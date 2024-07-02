use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use clickhouse::{Client, Row};
use serde::Deserialize;
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

        let database = if let Some(first_part) =
            url.path_segments().and_then(|mut segments| segments.next())
        {
            first_part
        } else {
            ""
        };

        let new_url = format!(
            "http://{}:{}",
            url.host().unwrap_or(Host::Domain("localhost")),
            url.port().unwrap_or(8443)
        );
        println!("{}", new_url);

        let mut client = Client::default().with_url(new_url).with_database(database);

        let user = url.username();
        let password = url.password();

        if let (u, Some(p)) = (user, password) {
            println!("{}", u);
            println!("{}", p);
            client = client.with_user(u).with_password(p);
        }

        #[derive(Row, Deserialize)]
        struct MyRow {
            field: String,
        }

        let mut cursor = client.query("SELECT 1").fetch::<MyRow>()?;
        while let Some(row) = cursor.next().await? {
            println!("{}", row.field);
        }

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
