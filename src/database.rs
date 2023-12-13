use anyhow::{bail, Result};
use async_trait::async_trait;

#[async_trait]
pub trait DatabaseDriver {
    async fn execute(self, content: &str) -> Result<()>;
}

pub fn new(db_url: &str) -> Result<impl DatabaseDriver> {
    if db_url.starts_with("libsql") {
        // TODO: return LibSQL database client
    }

    bail!("No matching database")
}
