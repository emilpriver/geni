use crate::config;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub mod libsql;
pub mod mysql;
pub mod postgres;
pub mod sqlite;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SchemaMigration {
    pub id: String,
}

// DatabaseDriver is a trait that all database drivers must implement
pub trait DatabaseDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>>;

    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;
}

// Creates a new database driver based on the database_url
pub async fn new(
    driver: config::Database,
    db_url: &str,
) -> Result<Box<dyn DatabaseDriver>, anyhow::Error> {
    // the database_url is starting with "libsql" if the libsql driver is used
    match driver {
        config::Database::LibSQL => {
            let token = config::database_token()?;
            let driver = libsql::LibSQLDriver::new(db_url, &token).await?;
            Ok(Box::new(driver))
        }
        config::Database::Postgres => {
            let driver = postgres::PostgresDriver::new(db_url).await?;
            Ok(Box::new(driver))
        }
        config::Database::MySQL => {
            let driver = mysql::MySQLDriver::new(db_url).await?;
            Ok(Box::new(driver))
        }
        config::Database::SQLite => {
            let driver = sqlite::SqliteDriver::new(db_url).await?;
            Ok(Box::new(driver))
        }
        config::Database::MariaDB => unimplemented!("MariaDB driver not implemented"),
    }
}
