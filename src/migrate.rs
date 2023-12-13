use crate::config::database_url;
use crate::config::migration_folder;
use crate::database_drivers;
use anyhow::{bail, Result};
use clap::ArgMatches;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

fn get_paths(folder: &PathBuf, ending: &str) -> Vec<PathBuf> {
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

    migration_files
}

fn read_file_content(path: &PathBuf) -> String {
    let content = fs::read_to_string(path).unwrap();

    content.replace("\n", "")
}

pub async fn up(query_matches: &ArgMatches) -> Result<()> {
    let folder = migration_folder();
    let path = PathBuf::from(&folder);
    let files = get_paths(&path, "up");

    if files.len() == 0 {
        bail!(
            "Didn't find any files ending with .up.sql at {}. Does the path exist?",
            folder
        );
    }

    let database_url = match database_url(Some(query_matches)) {
        Ok(d) => d,
        Err(err) => bail!("{}", err),
    };

    let database = database_drivers::new(database_url).unwrap();

    let migrations = database.get_or_create_schema_migrations().await?;

    for f in files {
        let id = f
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .split_once("_")
            .unwrap()
            .0
            .to_string();

        if !migrations.contains(&id) {
            let query = read_file_content(&f);
            println!("{:?}", query);
        }
    }

    Ok(())
}
