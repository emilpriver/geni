use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;
use std::vec;

pub fn get_local_migrations(folder: &PathBuf, ending: &str) -> Result<Vec<(i64, PathBuf)>> {
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

pub fn read_file_content(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}
