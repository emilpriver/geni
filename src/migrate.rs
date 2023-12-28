use crate::config::{database_url, migration_folder};
use crate::database_drivers;
use anyhow::{bail, Result};
use log::info;
use std::fs;
use std::path::PathBuf;
use std::vec;

fn get_migration_paths(folder: &PathBuf, ending: &str) -> Result<Vec<(i64, PathBuf)>> {
    let entries = match fs::read_dir(folder) {
        Ok(entries) => entries,
        Err(err) => {
            bail!("{:?}", err)
        }
    };

    let mut migration_files = vec![];
    let end = format!(".{}.sql", ending);

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if entry.file_name().to_str().unwrap().ends_with(&end) {
            migration_files.push((path.clone(), path));
        }
    }

    let mut sorted = migration_files
        .iter()
        .map(|(path, _)| {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let timestamp = filename.split_once('_').unwrap().0;
            let timestamp = timestamp.parse::<i64>().unwrap();

            (timestamp, path.clone())
        })
        .collect::<Vec<(i64, PathBuf)>>();

    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    Ok(sorted)
}

fn read_file_content(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

pub async fn up() -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = match get_migration_paths(&path, "up") {
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

    Ok(())
}

pub async fn down(rollback_amount: &i64) -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = match get_migration_paths(&path, "down") {
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

    Ok(())
}
