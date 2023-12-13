use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use async_trait::async_trait;
use libsql_client::{
    de,
    http::{Client, InnerClient},
    Config,
};

pub struct LibSQLDriver {
    db: Client,
}

impl LibSQLDriver {
    pub fn new(db_url: &str, token: &str) -> Result<LibSQLDriver> {
        let inner_client = InnerClient::Default;
        let config = Config::new(db_url)?.with_auth_token(token);

        let client = match Client::from_config(inner_client, config) {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };

        Ok(LibSQLDriver { db: client })
    }
}

#[async_trait]
impl DatabaseDriver for LibSQLDriver {
    // execute query with the provided database
    async fn execute(self, content: &str) -> Result<()> {
        match self.db.execute(content).await {
            Ok(..) => Ok(()),
            Err(err) => bail!("{:?}", err),
        }
    }

    // execute query with the provided database
    async fn get_or_create_schema_migrations(self) -> Result<Vec<String>> {
        let res = match self
            .db
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
