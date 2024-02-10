use crate::database_drivers;
use crate::utils::{get_local_migrations, read_file_content};
use anyhow::{bail, Result};
use log::info;
use std::path::PathBuf;

pub async fn up(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    dump_schema: bool,
) -> Result<()> {
    let path = PathBuf::from(&migration_folder);
    let files = match get_local_migrations(&path, "up") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .up.sql at {}. Does the path exist?",
            migration_folder,
        );
    }

    let mut database = database_drivers::new(
        database_url,
        database_token,
        migration_table,
        migration_folder.clone(),
        schema_file,
        wait_timeout,
        true,
    )
    .await?;

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
            let run_in_transaction = query
                .split_once('\n')
                // by default do we want to run in a transaction
                .unwrap_or(("transaction: yes", ""))
                .0
                .contains("transaction: yes");

            database.execute(&query, run_in_transaction).await?;

            database.insert_schema_migration(&id).await?;
        }
    }

    if dump_schema {
        if let Err(err) = database.dump_database_schema().await {
            log::error!("Skipping dumping database schema: {:?}", err);
        }
    }

    Ok(())
}

pub async fn down(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    dump_schema: bool,
    rollback_amount: &i64,
) -> Result<()> {
    let path = PathBuf::from(&migration_folder);
    let files = match get_local_migrations(&path, "down") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .down.sql at {}. Does the path exist?",
            migration_folder
        );
    }

    let mut database = database_drivers::new(
        database_url,
        database_token,
        migration_table,
        migration_folder.clone(),
        schema_file,
        wait_timeout,
        true,
    )
    .await?;

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
                let run_in_transaction = query
                    .split_once('\n')
                    // by default do we want to run in a transaction
                    .unwrap_or(("transaction: yes", ""))
                    .0
                    .contains("transaction: yes");

                database.execute(&query, run_in_transaction).await?;

                database
                    .remove_schema_migration(migration.to_string().as_str())
                    .await?;
            }
        }
    }

    if dump_schema {
        if let Err(err) = database.dump_database_schema().await {
            log::error!("Skipping dumping database schema: {:?}", err);
        }
    }

    Ok(())
}
