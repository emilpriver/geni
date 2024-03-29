use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use log::{error, info};
use regex::Regex;
use sqlx::mysql::MySqlRow;
use sqlx::Executor;
use sqlx::{Connection, MySqlConnection, Row};
use std::future::Future;
use std::pin::Pin;
use tokio::process::Command;

use super::utils;

pub struct MariaDBDriver {
    db: MySqlConnection,
    url: String,
    url_path: url::Url,
    db_name: String,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl<'a> MariaDBDriver {
    pub async fn new<'b>(
        db_url: &str,
        database_name: &str,
        wait_timeout: Option<usize>,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<MariaDBDriver> {
        let mut client = MySqlConnection::connect(db_url).await;

        let wait_timeout = wait_timeout.unwrap_or(0);

        if client.is_err() {
            let mut count = 0;
            loop {
                info!("Waiting for database to be ready");
                if count > wait_timeout {
                    bail!("Database is not ready");
                }

                match MySqlConnection::connect(db_url).await {
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

        let mut url_path = url::Url::parse(db_url)?;
        if url_path.host_str().unwrap() == "localhost" {
            url_path.set_host(Some("127.0.0.1"))?;
        }

        let m = MariaDBDriver {
            db: client.unwrap(),
            url: db_url.to_string(),
            db_name: database_name.to_string(),
            url_path,
            migrations_folder,
            migrations_table,
            schema_file,
        };

        Ok(m)
    }
}

impl DatabaseDriver for MariaDBDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
        run_in_transaction: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            if run_in_transaction {
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
                return Ok(());
            } else {
                self.db.execute(query).await?;
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
                self.migrations_table,
            );
            sqlx::query(query.as_str()).execute(&mut self.db).await?;
            let query = format!("SELECT id FROM {} ORDER BY id DESC", self.migrations_table);
            let result: Vec<String> = sqlx::query(query.as_str())
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
            let query = format!("INSERT INTO {} (id) VALUES (?)", self.migrations_table);
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
            let query = format!("DELETE FROM {} WHERE id = ?", self.migrations_table);
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
            if let Err(_err) = which::which("mariadb-dump") {
                bail!("mysqldump not found in PATH, is i installed?");
            };

            let host = format!("--host={}", self.url_path.host_str().unwrap());
            let username = format!("--user={}", self.url_path.username());
            let password = format!("--password={}", self.url_path.password().unwrap());
            let port = format!("--port={}", self.url_path.port().unwrap());

            let args: Vec<&str> = [
                "--opt",
                "--skip-dump-date",
                "--skip-add-drop-table",
                "--no-data",
                port.as_str(),
                host.as_str(),
                username.as_str(),
                password.as_str(),
                self.db_name.as_str(),
            ]
            .to_vec();

            let res = Command::new("mariadb-dump").args(args).output().await?;
            if !res.status.success() {
                bail!("mysqldump failed: {}", String::from_utf8_lossy(&res.stderr));
            }

            let schema = String::from_utf8_lossy(&res.stdout);

            let re = Regex::new(r"^/\*![0-9]{5}.*\*/").unwrap();

            let final_schema: String = schema.lines().filter(|line| !re.is_match(line)).fold(
                String::new(),
                |mut acc, line| {
                    acc.push_str(line);
                    acc.push('\n');
                    acc
                },
            );

            utils::write_to_schema_file(
                final_schema,
                self.migrations_folder.clone(),
                self.schema_file.clone(),
            )
            .await?;

            Ok(())
        };

        Box::pin(fut)
    }
}
