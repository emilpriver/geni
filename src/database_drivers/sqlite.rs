use crate::config;
use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use libsql_client::{de, local::Client};
use std::fs::{self, File};
use std::future::Future;
use std::pin::Pin;

use super::SchemaMigration;

pub struct SqliteDriver {
    db: Client,
    path: String,
}

impl<'a> SqliteDriver {
    pub async fn new<'b>(db_url: &str) -> Result<SqliteDriver> {
        let path = std::path::Path::new(db_url.split_once("://").unwrap().1);

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
        })
    }
}

impl DatabaseDriver for SqliteDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let queries = query
                .split(";")
                .map(|x| x.trim())
                .filter(|x| !x.is_empty())
                .collect::<Vec<&str>>();
            for query in queries {
                self.db.execute(query)?;
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
                config::migrations_table()
            );
            self.db.execute(query)?;

            let query = format!(
                "SELECT id FROM {} ORDER BY id DESC;",
                config::migrations_table()
            );

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
                    config::migrations_table(),
                    id,
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
                format!(
                    "DELETE FROM {} WHERE id = '{}';",
                    config::migrations_table(),
                    id
                )
                .as_str(),
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
            bail!("Geni does not support dumping a database, it should be done via the respective interface")
        };

        Box::pin(fut)
    }
}
