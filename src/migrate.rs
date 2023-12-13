use crate::config::migration_folder;
use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;

fn get_paths(folder: &PathBuf, ending: &str) -> Vec<PathBuf> {
    let entries = fs::read_dir(folder).unwrap();

    let mut migration_files = vec![];

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().unwrap().to_str().unwrap() == format!("{ending}.sql") {
            migration_files.push(path);
        }
    }

    migration_files
}

pub fn up() -> Result<()> {
    let folder = migration_folder();
    let path = PathBuf::from(&folder);
    let files = get_paths(&path, "up");

    if files.len() == 0 {
        bail!(
            "Didn't find any files ending with .up.sql at {}. Does the path exist?",
            folder
        );
    }

    for f in files {}

    Ok(())
}

