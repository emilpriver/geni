use anyhow::Result;
use chrono::Utc;
use log::info;
use std::fs::{self, File};
use std::io::Write;

pub fn generate_new_migration(migration_folder: &String, migration_name: &str) -> Result<()> {
    let timestamp = Utc::now().timestamp();
    let name = migration_name.replace(' ', "_").to_lowercase();

    let file_endings = vec!["up", "down"];

    for f in file_endings {
        let filename = format!("{migration_folder}/{timestamp}_{name}.{f}.sql");
        let filename_str = filename.as_str();
        let path = std::path::Path::new(filename_str);

        // Generate the folder if it don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;

        file.write_all(format!("-- Write your {f} sql migration here").as_bytes())?;

        info!("Generated {}", filename_str)
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_migration() {
        let migration_name = "Create users table";

        let tmp_dir = tempdir().unwrap();
        let migration_folder = tmp_dir.path();
        let migration_folder_string = migration_folder.to_str().unwrap().to_string();

        let result = generate_new_migration(&migration_folder_string, migration_name);

        assert!(result.is_ok());

        let timestamp = Utc::now().timestamp();
        let name = migration_name.replace(' ', "_").to_lowercase();

        let up_file = format!("{migration_folder_string}/{timestamp}_{name}.up.sql");
        let down_file = format!("{migration_folder_string}/{timestamp}_{name}.down.sql");

        assert!(fs::metadata(&up_file).is_ok());
        assert!(fs::metadata(&down_file).is_ok());

        let up_contents = fs::read_to_string(&up_file).unwrap();
        assert!(up_contents.contains("Write your up sql migration here"));

        let down_contents = fs::read_to_string(&down_file).unwrap();
        assert!(down_contents.contains("Write your down sql migration here"));
    }
}
