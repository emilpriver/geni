use crate::config;
use anyhow::{bail, Result};
use async_trait::async_trait;

pub mod libsql;

#[async_trait]
pub trait DatabaseDriver {
    async fn execute(self, content: &str) -> Result<()>;
    async fn get_or_create_schema_migrations(self) -> Result<Vec<String>>;
}

pub fn new(db_url: &str) -> Result<Box<dyn DatabaseDriver>> {
    if db_url.starts_with("libsql") {
        let token = match config::database_token() {
            Ok(t) => t,
            Err(err) => bail!("{}", err),
        };

        let client = match libsql::LibSQLDriver::new(db_url, token.as_str()) {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };

        return Ok(Box::new(client));
    }

    bail!("No matching database")
}
