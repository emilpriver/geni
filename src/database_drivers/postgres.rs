use crate::config;
use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use log::{error, info};
use sqlx::postgres::PgRow;
use sqlx::Executor;
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
        let mut client = PgConnection::connect(db_url).await;

        let wait_timeout = config::wait_timeout();

        if client.is_err() {
            let mut count = 0;
            loop {
                info!("Waiting for database to be ready");
                if count > wait_timeout {
                    bail!("Database is not ready");
                }

                match PgConnection::connect(db_url).await {
                    Ok(c) => {
                        client = Ok(c);
                        break;
                    }
                    Err(_) => {
                        count += 1;
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
        }

        let mut p = PostgresDriver {
            db: client.unwrap(),
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
            let mut tx = self.db.begin().await?;

            match tx.execute(query).await {
                Ok(_) => {
                    tx.commit().await?;
                }
                Err(e) => {
                    error!("Error executing query: {}", e);
                    tx.rollback().await?;
                }
            }

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

    fn dump_database_schema(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::prelude::v1::Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            bail!("Geni does not support dumping a database, it should be done via the respective interface")
        };

        Box::pin(fut)
    }
}
