use crate::config::database_url;
use crate::config::migration_folder;
use crate::database_drivers;
use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;
use std::vec;

fn get_paths(folder: &PathBuf, ending: &str) -> Vec<(i64, PathBuf)> {
    let entries = fs::read_dir(folder).unwrap();

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
            let timestamp = filename.split_once("_").unwrap().0;
            let timestamp = timestamp.parse::<i64>().unwrap();

            (timestamp, path.clone())
        })
        .collect::<Vec<(i64, PathBuf)>>();

    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    sorted
}

fn read_file_content(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

pub async fn up() -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = get_paths(&path, "up");

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .up.sql at {}. Does the path exist?",
            folder
        );
    }

    let database_url = database_url()?;
    let database = database_drivers::new(&database_url).await?;

    let migrations: Vec<String> = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.id.into_boxed_str())
        .collect::<Vec<Box<str>>>()
        .into_iter()
        .map(|s| s.into())
        .collect();

    for f in files {
        let id = Box::new(f.0.to_string());

        if !migrations.contains(&id) {
            println!("Running migration {}", id);
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
    let files = get_paths(&path, "down");

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .down.sql at {}. Does the path exist?",
            folder
        );
    }

    let database_url = database_url()?;
    let database = database_drivers::new(&database_url).await?;

    let migrations = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.id.into_boxed_str())
        .collect::<Vec<Box<str>>>();

    panic!("Migrations: {:?}", migrations);
    // FIXME: Allow to rollback more the 1 migration

    let last_migration = migrations.last().unwrap();
    let last_migration_int = last_migration.parse::<i64>().unwrap();

    let rollback_file = files
        .iter()
        .find(|(timestamp, _)| timestamp == &last_migration_int);

    match rollback_file {
        None => bail!("No rollback file found for {}", last_migration_int),
        Some(f) => {
            println!("Running rollback for {}", last_migration_int);
            let query = read_file_content(&f.1);

            database.execute(&query).await?;

            database.remove_schema_migration(&last_migration).await?;
        }
    }

    Ok(())
}
