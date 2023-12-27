use crate::config;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub mod libsql;
pub mod maria;
pub mod mysql;
pub mod postgres;
pub mod sqlite;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SchemaMigration {
    pub id: String,
}

// DatabaseDriver is a trait that all database drivers must implement
pub trait DatabaseDriver {
    // execute giving query for the specifiv
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // create database with the specific driver
    fn create(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // drop database with the specific driver
    fn drop(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // get current schema migrations for the schema migrations table
    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>>;

    // insert new schema migration
    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // remove schema migration from the schema migrations table
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
    let parsed_db_url = url::Url::parse(db_url)?;

    let scheme = parsed_db_url.scheme();

    match scheme {
        "libsql" => {
            let token = config::database_token();
            let driver = libsql::LibSQLDriver::new(db_url, token).await?;
            Ok(Box::new(driver))
        }
        "postgres" => {
            let driver = postgres::PostgresDriver::new(db_url).await?;
            Ok(Box::new(driver))
        }
        "mysql" => {
            let driver = mysql::MySQLDriver::new(db_url).await?;
            Ok(Box::new(driver))
        }
        "sqlite" => {
            let driver = sqlite::SqliteDriver::new(db_url).await?;
            Ok(Box::new(driver))
        }
        "mariadb" => {
            let driver = maria::MariaDBDriver::new(db_url).await?;
            Ok(Box::new(driver))
        }
        _ => bail!("Unsupported database driver: {}", scheme),
    }
}
