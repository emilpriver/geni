use crate::config;
use std::future::Future;

pub mod libsql;

// DatabaseDriver is a trait that all database drivers must implement
pub trait DatabaseDriver<'a> {
    fn execute(
        &'a self,
        query: &str,
    ) -> Box<dyn Future<Output = Result<(), anyhow::Error>> + Unpin>;
    fn get_or_create_schema_migrations(
        &'a self,
    ) -> Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + Unpin>;
}

// TODO: add Clone to DatabaseDriver

// Creates a new database driver based on the database_url
pub async fn new(db_url: &str) -> Result<Box<dyn DatabaseDriver>, anyhow::Error> {
    // the database_url is starting with "libsql" if the libsql driver is used
    if db_url.starts_with("libsql") {
        let token = config::database_token()?;

        let client = libsql::LibSQLDriver::new(db_url, token.as_str()).await?;

        return Ok(Box::new(client));
    }

    unimplemented!("Database driver not implemented")
}
