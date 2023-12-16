use std::sync::Arc;

use crate::config;
use anyhow::{bail, Result};
use async_trait::async_trait;
use serde::Serialize;

pub mod libsql;
pub mod postgres;

#[async_trait]
pub trait DatabaseDriver {
    async fn execute(&self, query: &str) -> Result<()>;
    async fn get_or_create_schema_migrations(&self) -> Result<Vec<String>>;
}

fn print_serialized_trait_object<T: Serialize>(t: &T) {
    let serialized = serde_json::to_string(t).unwrap();
    println!("{}", serialized);
}

pub async fn new(db_url: &str) -> Result<Arc<dyn DatabaseDriver>> {
    if db_url.starts_with("libsql") {
        let token = match config::database_token() {
            Ok(t) => t,
            Err(err) => bail!("{}", err),
        };

        let client = match libsql::LibSQLDriver::new(db_url, token.as_str()).await {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };

        return Ok(Arc::new(client));
    }

    if db_url.starts_with("postgressql") {
        let client = match postgres::PostgresDriver::new(db_url).await {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };

        return Ok(Arc::new(client));
    }

    bail!("No matching database")
}
