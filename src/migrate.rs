use crate::config::{self, database_url, migration_folder};
use crate::database_drivers;
use crate::utils::{get_local_migrations, read_file_content};
use anyhow::{bail, Result};
use log::info;
use std::path::PathBuf;

pub async fn up() -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = match get_local_migrations(&path, "up") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .up.sql at {}. Does the path exist?",
            folder
        );
    }

    let database_url = database_url()?;
    let mut database = database_drivers::new(&database_url, true).await?;

    let migrations: Vec<String> = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.into_boxed_str())
        .collect::<Vec<Box<str>>>()
        .into_iter()
        .map(|s| s.into())
        .collect();

    for f in files {
        let id = Box::new(f.0.to_string());

        if !migrations.contains(&id) {
            info!("Running migration {}", id);
            let query = read_file_content(&f.1);

            database.execute(&query).await?;

            database.insert_schema_migration(&id).await?;
        }
    }

    if config::dump_schema_file() {
        if let Err(err) = database.dump_database_schema().await {
            log::error!("Skipping dumping database schema: {:?}", err);
        }
    }

    Ok(())
}

pub async fn down(rollback_amount: &i64) -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = match get_local_migrations(&path, "down") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .down.sql at {}. Does the path exist?",
            folder
        );
    }

    let database_url = database_url()?;
    let mut database = database_drivers::new(&database_url, true).await?;

    let migrations = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.into_boxed_str().parse::<i64>().unwrap())
        .collect::<Vec<i64>>();

    let migrations_to_run = migrations.into_iter().take(*rollback_amount as usize);

    for migration in migrations_to_run {
        let rollback_file = files.iter().find(|(timestamp, _)| timestamp == &migration);

        match rollback_file {
            None => bail!("No rollback file found for {}", migration),
            Some(f) => {
                info!("Running rollback for {}", migration);
                let query = read_file_content(&f.1);

                database.execute(&query).await?;

                database
                    .remove_schema_migration(migration.to_string().as_str())
                    .await?;
            }
        }
    }

    if config::dump_schema_file() {
        if let Err(err) = database.dump_database_schema().await {
            log::error!("Skipping dumping database schema: {:?}", err);
        }
    }

    Ok(())
}
