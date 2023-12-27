use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use libsql_client::{de, Client, Config};
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
            self.db.execute(query).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move {
            let query =
                "CREATE TABLE IF NOT EXISTS schema_migrations (id VARCHAR(255) PRIMARY KEY);";
            self.db.execute(query).await?;
            let query = "SELECT id FROM schema_migrations ORDER BY id DESC;";
            let result = self.db.execute(query).await?;

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
                .execute(format!("INSERT INTO schema_migrations (id) VALUES ('{}');", id).as_str())
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
                .execute(format!("DELETE FROM schema_migrations WHERE id = '{}';", id).as_str())
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
}
