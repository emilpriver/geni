use std::{
    os::fd::AsFd,
    sync::{Arc, Mutex},
};

use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use async_trait::async_trait;
use libsql_client::{de, Client, Config};

pub struct LibSQLDriver {
    db: Arc<Client>,
}

impl LibSQLDriver {
    pub async fn new(db_url: &str, token: &str) -> Result<LibSQLDriver> {
        let config = Config::new(db_url)?.with_auth_token(token);

        let client = match libsql_client::Client::from_config(config).await {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };

        Ok(LibSQLDriver {
            db: Arc::new(client),
        })
    }
}

#[async_trait]
impl DatabaseDriver for LibSQLDriver {
    /**
     * execute query with the provided database
     */
    async fn execute(&self, query: &str) -> Result<()> {
        let client: Arc<libsql_client::Client> = Arc::clone(&self.db);

        match client.execute(query).await {
            Ok(r) => Ok(()),
            Err(err) => bail!("{:?}", err),
        }
    }

    /**
     * execute query with the provided database
     */
    async fn get_or_create_schema_migrations(&self) -> Result<Vec<String>> {
        let client: Arc<libsql_client::Client> = Arc::clone(&self.db);
        let res = match client
            .execute(
                r"
                CREATE TABLE IF NOT EXISTS schema_migrations (
                    id VARCHAR NOT NULL PRIMARY KEY
                );

                SELECT id FROM schema_migrations;
            ",
            )
            .await
        {
            Ok(r) => r
                .rows
                .iter()
                .map(de::from_row)
                .collect::<Result<Vec<String>>>(),
            Err(err) => bail!("{:?}", err),
        };

        match res {
            Ok(r) => Ok(r),
            Err(err) => bail!("{:?}", err),
        }
    }
}
