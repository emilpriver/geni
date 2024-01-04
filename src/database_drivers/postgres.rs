use crate::config;
use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use log::{error, info};
use sqlx::postgres::PgRow;
use sqlx::Executor;
use sqlx::{Connection, PgConnection, Row};
use std::future::Future;
use std::pin::Pin;
use tokio::process::Command;

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
            let query = format!(
                "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) PRIMARY KEY)",
                config::migrations_table()
            );
            sqlx::query(query.as_str()).execute(&mut self.db).await?;

            let query = format!(
                "SELECT id FROM {} ORDER BY id DESC",
                config::migrations_table(),
            );

            let result: Vec<String> = sqlx::query(query.as_str())
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
            let query = format!(
                "INSERT INTO {} (id) VALUES ($1)",
                config::migrations_table()
            );
            sqlx::query(query.as_str())
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
            let query = format!("DELETE FROM {} WHERE id = $1", config::migrations_table());
            sqlx::query(query.as_str())
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
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            if which::which("pg_dump").is_err() {
                bail!("pg_dump not found in PATH, is i installed?");
            };

            let connection_string = format!("--dbname={}", self.url);

            let args: Vec<&str> = [
                "--format=plain",
                "--encoding=UTF8",
                "--schema-only",
                "--no-owner",
                "--no-privileges",
                "--format=plain",
                connection_string.as_str(),
            ]
            .to_vec();

            let res = Command::new("pg_dump").args(args).output().await?;
            if !res.status.success() {
                bail!("pg_dump failed: {}", String::from_utf8_lossy(&res.stderr));
            }

            let schema = String::from_utf8_lossy(&res.stdout);

            utils::write_to_schema_file(schema.to_string()).await?;

            Ok(())
        };

        Box::pin(fut)
    }
}
