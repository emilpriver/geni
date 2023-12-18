use crate::config::database_url;
use crate::config::migration_folder;
use crate::database_drivers;
use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;
use std::vec;

fn get_paths<'a>(folder: &'a PathBuf, ending: &'a str) -> Vec<(i64, &'a PathBuf)> {
    let entries = fs::read_dir(folder).unwrap();

    let mut migration_files = vec![];
    let end = format!(".{}.sql", ending);

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if entry.file_name().to_str().unwrap().ends_with(&end) {
            migration_files.push(path);
        }
    }

    let mut sorted = migration_files
        .iter()
        .map(|p| {
            let filename = p.file_name().unwrap().to_str().unwrap();
            let timestamp = filename.split_once("_").unwrap().0;
            let timestamp = timestamp.parse::<i64>().unwrap();

            (timestamp, p)
        })
        .collect::<Vec<(i64, &PathBuf)>>();

    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    sorted
}

fn read_file_content(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

pub enum MigrationType {
    Up,
    Down,
}

impl MigrationType {
    pub fn to_str(&self) -> &str {
        match self {
            MigrationType::Up => "up",
            MigrationType::Down => "down",
        }
    }
}

pub async fn migrate(migration_type: MigrationType) -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = get_paths(&path, migration_type.to_str());

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .{}.sql at {}. Does the path exist?",
            migration_type.to_str(),
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

            match migration_type {
                MigrationType::Up => database.insert_schema_migration(&id).await?,
                MigrationType::Down => database.remove_schema_migration(&id).await?,
            }
        }
    }

    Ok(())
}
