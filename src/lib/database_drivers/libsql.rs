use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use libsql_client::{de, Client, Config, Statement};
use std::future::Future;
use std::pin::Pin;

use super::{utils, SchemaMigration};

pub struct LibSQLDriver {
    db: Client,
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
        let mut config = Config::new(db_url.as_str())?;
        if let Some(token) = token {
            config = config.with_auth_token(token);
        }

        let client = match libsql_client::Client::from_config(config).await {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };

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
            let queries = query
                .split(';')
                .filter(|x| !x.trim().is_empty())
                .map(Statement::new)
                .collect::<Vec<Statement>>();

            if run_in_transaction {
                self.db.batch(queries).await?;
            } else {
                for query in queries {
                    match self.db.execute(query).await {
                        Ok(_) => {}
                        Err(e) => {
                            bail!("{:?}", e);
                        }
                    }
                }
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
                "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) NOT NULL PRIMARY KEY);",
                self.migrations_table,
            );
            self.db.execute(query).await?;
            let query = format!("SELECT id FROM {} ORDER BY id DESC;", self.migrations_table);
            let result = self.db.execute(query.as_str()).await?;

            let rows = result.rows.iter().map(de::from_row);

            let res = rows.collect::<Result<Vec<SchemaMigration>>>();

            res.map(|v| v.into_iter().map(|s| s.id).collect())
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
                        self.migrations_table, id
                    )
                    .as_str(),
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
            self.db.execute("SELECT 1").await?;

            Ok(())
        };

        Box::pin(fut)
    }

    fn dump_database_schema(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let res = self
                .db
                .execute("SELECT sql FROM sqlite_master WHERE type='table'")
                .await?;

            let final_schema = res
                .rows
                .iter()
                .map(|row| {
                    row.values
                        .iter()
                        .map(|v| {
                            let m = v
                                .to_string()
                                .trim_start_matches('"')
                                .trim_end_matches('"')
                                .to_string()
                                .replace("\\n", "\n");

                            format!("{};", m)
                        })
                        .collect::<Vec<String>>()
                        .join("\n")
                })
                .collect::<Vec<String>>()
                .join("\n");

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
