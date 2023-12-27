use std::fs::{self, File};
use std::io::Write;

use anyhow::Result;
use chrono::Utc;

use crate::config::migration_folder;

pub fn generate_new_migration(migration_name: &str) -> Result<()> {
    let timestamp = Utc::now().timestamp();
    let name = migration_name.replace(' ', "_").to_lowercase();

    let file_endings = vec!["up", "down"];

    let migration_path = migration_folder();

    for f in file_endings {
        let filename = format!("{migration_path}/{timestamp}_{name}.{f}.sql");
        let filename_str = filename.as_str();
        let path = std::path::Path::new(filename_str);

        // Generate the folder if it don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(path)?;

        file.write_all(format!("-- Write your {f} sql migration here").as_bytes())?;

        println!("Generated {}", filename_str)
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::env;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_migration() {
        let migration_name = "Create users table";

        let tmp_dir = tempdir().unwrap();
        let migration_folder = tmp_dir.path();
        let migration_folder_string = migration_folder.to_str().unwrap();

        env::set_var("DATABASE_MIGRATIONS_FOLDER", migration_folder_string);

        let result = generate_new_migration(migration_name);

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
