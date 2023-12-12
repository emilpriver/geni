use std::fs::{self, File};
use std::io::Write;

use anyhow::{bail, Result};
use chrono::Utc;

use crate::config::migration_folder;

pub fn generate_new_migration(migration_name: &str) -> Result<()> {
    let timestamp = Utc::now().timestamp();
    let name = migration_name.replace(" ", "_").to_lowercase();

    let file_endings = vec!["up", "down"];

    let migration_path = migration_folder();

    for f in file_endings {
        let filename = format!("{migration_path}/{timestamp}_{name}.{f}.sql");
        let filename_str = &filename.as_str();
        let path = std::path::Path::new(filename_str);

        // Generate the folder id it don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = match File::create(path) {
            Ok(f) => f,
            Err(err) => bail!(err),
        };
        match file.write_all(format!("Write your {f} sql migration here").as_bytes()) {
            Err(err) => bail!(err),
            _ => {}
        };
    }

    Ok(())
}
