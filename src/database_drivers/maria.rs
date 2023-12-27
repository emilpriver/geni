use crate::config;
use crate::database_drivers::DatabaseDriver;
use anyhow::{Result};
use sqlx::mysql::MySqlRow;
use sqlx::{Connection, MySqlConnection, Row};
use std::future::Future;
use std::pin::Pin;

pub struct MariaDBDriver {
    db: MySqlConnection,
    url: String,
    db_name: String,
}

impl<'a> MariaDBDriver {
    pub async fn new<'b>(db_url: &str, database_name: &str) -> Result<MariaDBDriver> {
        let mut client = MySqlConnection::connect(db_url).await?;

        let wait_timeout = config::wait_timeout();

        let mut count = 0;
        loop {
            if count > wait_timeout {
                break;
            }

            if client.ping().await.is_ok() {
                break;
            }

            count += 1;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Ok(MariaDBDriver {
            db: client,
            url: db_url.to_string(),
            db_name: database_name.to_string(),
        })
    }
}

impl DatabaseDriver for MariaDBDriver {
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
                .map(|row: MySqlRow| row.get("id"))
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
            sqlx::query("INSERT INTO schema_migrations (id) VALUES (?)")
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
            sqlx::query("delete from schema_migrations where id = ?")
                .bind(id)
                .execute(&mut self.db)
                .await?;

            Ok(())
        };

        Box::pin(fut)
    }

    fn create_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("CREATE DATABASE IF NOT EXISTS {}", self.db_name);

            let mut client = MySqlConnection::connect(self.url.as_str()).await?;
            sqlx::query(query.as_str()).execute(&mut client).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn drop_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("DROP DATABASE IF EXISTS {}", self.db_name);

            let mut client = MySqlConnection::connect(self.url.as_str()).await?;
            sqlx::query(query.as_str()).execute(&mut client).await?;
            Ok(())
        };

        Box::pin(fut)
    }
}
