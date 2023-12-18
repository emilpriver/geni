use crate::config;
use std::future::Future;
use std::pin::Pin;

pub mod libsql;

// DatabaseDriver is a trait that all database drivers must implement
pub trait DatabaseDriver {
    fn execute<'a>(
        &'a self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    fn get_or_create_schema_migrations(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>>;

    fn insert_schema_migration<'a>(
        &'a self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    fn remove_schema_migration<'a>(
        &'a self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;
}

// Creates a new database driver based on the database_url
pub async fn new(db_url: &str) -> Result<Box<dyn DatabaseDriver>, anyhow::Error> {
    // the database_url is starting with "libsql" if the libsql driver is used
    if db_url.contains("libsql://") {
        let token = config::database_token()?;

        let client = libsql::LibSQLDriver::new(db_url, token.as_str()).await?;

        return Ok(Box::new(client));
    }

    unimplemented!("Database driver not implemented")
}
