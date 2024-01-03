use super::DatabaseDriver;
use crate::config;
use anyhow::{bail, Result};
use log::info;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub async fn wait_for_database(client: &mut dyn DatabaseDriver) -> Result<()> {
    let wait_timeout = config::wait_timeout();

    let mut count = 0;
    loop {
        info!("Waiting for database to be ready");
        if count > wait_timeout {
            bail!("Database is not ready");
        }

        if client.ready().await.is_ok() {
            break;
        }

        count += 1;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}

pub async fn write_to_schema_file(content: String) -> Result<()> {
    let schema_path = format!("{}/schema.sql", config::migration_folder());
    let path = Path::new(schema_path.as_str());

    if File::open(path.to_str().unwrap()).is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        File::create(&schema_path)?;
    };

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(schema_path.as_str())?;

    file.write_all(content.as_bytes())?;
    Ok(())
}
