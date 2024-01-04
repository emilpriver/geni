use crate::config;
use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use libsql_client::{de, Client, Config, Statement};
use std::future::Future;
use std::pin::Pin;

use super::SchemaMigration;

pub struct LibSQLDriver {
    db: Client,
}

impl<'a> LibSQLDriver {
    pub async fn new<'b>(db_url: &str, token: Option<String>) -> Result<LibSQLDriver> {
        let mut config = Config::new(db_url)?;
        if let Some(token) = token {
            config = config.with_auth_token(token);
        }

        let client = match libsql_client::Client::from_config(config).await {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };

        Ok(LibSQLDriver { db: client })
    }
}

impl DatabaseDriver for LibSQLDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let queries = query
                .split(";")
                .filter(|x| !x.trim().is_empty())
                .map(|x| Statement::new(x))
                .collect::<Vec<Statement>>();

            self.db.batch(queries).await?;

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
                config::migrations_table()
            );
            self.db.execute(query).await?;
            let query = format!(
                "SELECT id FROM {} ORDER BY id DESC;",
                config::migrations_table()
            );
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
                        config::migrations_table(),
                        id
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
                    format!(
                        "DELETE FROM {} WHERE id = '{}';",
                        config::migrations_table(),
                        id
                    )
                    .as_str(),
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
            bail!("Geni does not support dumping a database, it should be done via the respective interface")
        };

        Box::pin(fut)
    }
}
