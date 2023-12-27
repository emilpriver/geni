use crate::config;
use crate::database_drivers::DatabaseDriver;
use anyhow::Result;
use sqlx::postgres::PgRow;
use sqlx::{Connection, PgConnection, Row};
use std::future::Future;
use std::pin::Pin;

use super::utils;

pub struct PostgresDriver {
    db: PgConnection,
    url: String,
    db_name: String,
}

impl<'a> PostgresDriver {
    pub async fn new<'b>(db_url: &str, database_name: &str) -> Result<PostgresDriver> {
        let client = PgConnection::connect(db_url).await?;

        let mut p = PostgresDriver {
            db: client,
            url: db_url.to_string(),
            db_name: database_name.to_string(),
        };

        utils::wait_for_database(&mut p).await?;

        Ok(p)
    }
}

impl DatabaseDriver for PostgresDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query(query).execute(&mut self.db).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move {
            let query =
                "CREATE TABLE IF NOT EXISTS schema_migrations (id VARCHAR(255) PRIMARY KEY)";
            sqlx::query(query).execute(&mut self.db).await?;
            let query = "SELECT id FROM schema_migrations ORDER BY id DESC";
            let result: Vec<String> = sqlx::query(query)
                .map(|row: PgRow| row.get("id"))
                .fetch_all(&mut self.db)
                .await?;

            Ok(result)
        };

        Box::pin(fut)
    }

    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query("INSERT INTO schema_migrations (id) VALUES ($1)")
                .bind(id)
                .execute(&mut self.db)
                .await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query("delete from schema_migrations where id = $1")
                .bind(id)
                .execute(&mut self.db)
                .await?;

            Ok(())
        };

        Box::pin(fut)
    }

    fn create_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("CREATE DATABASE {}", self.db_name);

            let mut client = PgConnection::connect(self.url.as_str()).await?;
            sqlx::query(query.as_str()).execute(&mut client).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn drop_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("DROP DATABASE {}", self.db_name);

            let mut client = PgConnection::connect(self.url.as_str()).await?;
            sqlx::query(query.as_str()).execute(&mut client).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn ready(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query("SELECT 1").execute(&mut self.db).await?;
            Ok(())
        };

        Box::pin(fut)
    }
}
