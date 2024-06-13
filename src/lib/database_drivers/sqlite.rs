use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use libsql_client::{de, local::Client};
use std::fs::{self, File};
use std::future::Future;
use std::pin::Pin;

use super::{utils, SchemaMigration};

pub struct SqliteDriver {
    db: Client,
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

        let client = Client::new(path.to_str().unwrap()).unwrap();

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
                self.db.execute("BEGIN;")?;
            }

            let queries = query
                .split(';')
                .map(|x| x.trim())
                .filter(|x| !x.is_empty())
                .collect::<Vec<&str>>();
            for query in queries {
                match self.db.execute(query) {
                    Ok(_) => {}
                    Err(e) => {
                        if run_in_transaction {
                            self.db.execute("ROLLBACK;")?;
                        }
                        bail!("{:?}", e);
                    }
                }
            }

            if run_in_transaction {
                self.db.execute("COMMIT;")?;
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
            self.db.execute(query)?;

            let query = format!("SELECT id FROM {} ORDER BY id DESC;", self.migrations_table);

            let result = self.db.execute(query)?;

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
            self.db.execute(
                format!(
                    "INSERT INTO {} (id) VALUES ('{}');",
                    self.migrations_table, id,
                )
                .as_str(),
            )?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            self.db.execute(
                format!("DELETE FROM {} WHERE id = '{}';", self.migrations_table, id).as_str(),
            )?;
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
            let res = self.db.execute("SELECT sql FROM sqlite_master ")?;
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
