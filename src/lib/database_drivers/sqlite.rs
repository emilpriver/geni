use crate::database_drivers::DatabaseDriver;
use anyhow::Result;

use libsql::{params, Builder, Connection};
use std::fs::{self, File};
use std::future::Future;
use std::pin::Pin;

use super::utils;

pub struct SqliteDriver {
    db: Connection,
    path: String,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl<'a> SqliteDriver {
    pub async fn new<'b>(
        db_url: &str,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<SqliteDriver> {
        let path = if db_url.contains("://") {
            std::path::Path::new(db_url.split_once("://").unwrap().1)
        } else {
            std::path::Path::new(db_url)
        };

        if File::open(path.to_str().unwrap()).is_err() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            File::create(path)?;
        }
        let client = Builder::new_local(path).build().await?.connect()?;

        Ok(SqliteDriver {
            db: client,
            path: path.to_str().unwrap().to_string(),
            migrations_table,
            migrations_folder,
            schema_file,
        })
    }
}

impl DatabaseDriver for SqliteDriver {
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
            let query = format!(
                "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) PRIMARY KEY);",
                self.migrations_table
            );
            self.db.execute(query.as_str(), params![]).await?;

            let query = format!("SELECT id FROM {} ORDER BY id DESC;", self.migrations_table);
            let mut result = self.db.query(query.as_str(), params![]).await?;

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
            self.db
                .execute(
                    format!(
                        "INSERT INTO {} (id) VALUES ('{}');",
                        self.migrations_table, id,
                    )
                    .as_str(),
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
            self.db
                .execute(
                    format!("DELETE FROM {} WHERE id = '{}';", self.migrations_table, id).as_str(),
                    params![],
                )
                .await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn create_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { Ok(()) };

        Box::pin(fut)
    }

    fn drop_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            fs::remove_file(&mut self.path)?;

            Ok(())
        };

        Box::pin(fut)
    }

    // SQlite don't have a HTTP connection so we don't need to check if it's ready
    fn ready(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { Ok(()) };

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
                }
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
