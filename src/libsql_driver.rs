use crate::database::DatabaseDriver;
use anyhow::{bail, Result};
use async_trait::async_trait;
use libsql_client::{
    http::{Client, InnerClient},
    Config,
};

struct LibSQLDriver {
    db: Client,
}

impl LibSQLDriver {
    fn new(db_url: &str, token: &str) -> Result<LibSQLDriver> {
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
}
