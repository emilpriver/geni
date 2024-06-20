use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use libsql::{params, Builder, Connection};
use log::info;
use std::future::Future;
use std::pin::Pin;

use super::utils;

pub struct LibSQLDriver {
    db: Connection,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl<'a> LibSQLDriver {
    pub async fn new<'b>(
        db_url: &String,
        token: Option<String>,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<LibSQLDriver> {
        let db = if !db_url.starts_with("libsql://./") {
            let auth_token = if let Some(t) = token {
                t
            } else {
                info!("Token is not set, using empty string");
                "".to_string()
            };

            Builder::new_remote(db_url.to_owned(), auth_token)
                .build()
                .await
                .unwrap()
        } else {
            bail!("libsql:// should only be used with remote database. Use sqlite:// protocol when running local sqlite files")
        };

        let client = db.connect()?;

        Ok(LibSQLDriver {
            db: client,
            migrations_folder,
            migrations_table,
            schema_file,
        })
    }
}

impl DatabaseDriver for LibSQLDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
        run_in_transaction: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            if run_in_transaction {
                self.db.execute_transactional_batch(query).await?;
            } else {
                self.db.execute_batch(query).await?;
            }

            Ok(())
        };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move {
            self.db
                .execute(
                    format!(
                        "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) NOT NULL PRIMARY KEY);",
                        self.migrations_table
                    )
                    .as_str(),
                    params![],
                )
                .await?;

            let mut result = self
                .db
                .query(
                    format!(
                        " SELECT id FROM {} ORDER BY id DESC;",
                        self.migrations_table
                    )
                    .as_str(),
                    params![],
                )
                .await?;

            let mut schema_migrations: Vec<String> = vec![];
            while let Some(row) = result.next().await.unwrap() {
                if let Ok(r) = row.get_str(0) {
                    schema_migrations.push(r.to_string());
                    continue;
                }
                break;
            }

            Ok(schema_migrations)
        };

        Box::pin(fut)
    }

    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let migrations_table = self.migrations_table.as_str();
            self.db
                .execute(
                    format!("INSERT INTO {} (id) VALUES ('{}')", migrations_table, id).as_str(),
                    params![],
                )
                .await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let migrations_table = self.migrations_table.as_str();
            self.db
                .execute(
                    format!("DELETE FROM {} WHERE id = '{}'", migrations_table, id,).as_str(),
                    params![],
                )
                .await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn create_database(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::prelude::v1::Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            bail!("Geni does not support creating a database, it should be done via the respective interface")
        };

        Box::pin(fut)
    }

    fn drop_database(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::prelude::v1::Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            bail!("Geni does not support dropping a database, it should be done via the respective interface")
        };

        Box::pin(fut)
    }

    // SQlite don't have a HTTP connection so we don't need to check if it's ready
    fn ready(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            self.db.execute("SELECT 1", params![]).await?;

            Ok(())
        };

        Box::pin(fut)
    }

    fn dump_database_schema(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let mut result = self
                .db
                .query("SELECT sql FROM sqlite_master", params![])
                .await?;

            let mut schemas: Vec<String> = vec![];
            while let Some(row) = result.next().await.unwrap_or(None) {
                if let Ok(r) = row.get_str(0) {
                    schemas.push(
                        r.to_string()
                            .trim_start_matches('"')
                            .trim_end_matches('"')
                            .to_string()
                            .replace("\\n", "\n"),
                    );
                    continue;
                }

                break;
            }

            let final_schema = schemas.join("\n");

            utils::write_to_schema_file(
                final_schema,
                self.migrations_folder.clone(),
                self.schema_file.clone(),
            )
            .await?;

            Ok(())
        };

        Box::pin(fut)
    }
}
