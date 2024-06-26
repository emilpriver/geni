use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use clickhouse::Client;
use url::{Host, Url};

use std::future::Future;
use std::pin::Pin;

pub struct ClickhouseDriver {
    db: Client,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl<'a> ClickhouseDriver {
    pub async fn new<'b>(
        db_url: &str,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<ClickhouseDriver> {
        let url = if let Ok(u) = Url::parse(db_url) {
            u
        } else {
            bail!("Invalid url");
        };

        let client = Client::default()
            .with_url(format!(
                "{}://{}:{}",
                url.scheme(),
                url.host().unwrap_or(Host::Domain("localhost")),
                url.port().unwrap_or(8443)
            ))
            .with_user("name")
            .with_password("123")
            .with_database("test");

        Ok(ClickhouseDriver {
            db: client,
            migrations_folder,
            migrations_table,
            schema_file,
        })
    }
}

impl DatabaseDriver for ClickhouseDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
        run_in_transaction: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }

    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }

    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }

    fn create_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }

    fn drop_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }

    fn ready(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }

    fn dump_database_schema(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move { todo!() };

        Box::pin(fut)
    }
}
