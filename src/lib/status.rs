use std::path::PathBuf;

use crate::{
    database_drivers,
    utils::{get_local_migrations, read_file_content},
};
use anyhow::{bail, Result};
use log::info;

pub async fn status(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    verbose: bool,
) -> Result<()> {
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

    let path = PathBuf::from(&migration_folder);
    let files = match get_local_migrations(&path, "up") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    let migrations: Vec<String> = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.into_boxed_str())
        .collect::<Vec<Box<str>>>()
        .into_iter()
        .map(|s| s.into())
        .collect();

    compare_migrations_and_log(files, migrations, verbose);

    Ok(())
}

// Extracted for easier testing
fn compare_migrations_and_log(files: Vec<(i64, PathBuf)>, migrations: Vec<String>, verbose: bool) {
    for f in files {
        let id = Box::new(f.0.to_string());

        if !migrations.contains(&id) {
            if verbose {
                let query = read_file_content(&f.1);
                info!("Pending migration {}: \n {}", id, query);
            } else {
                info!("Pending {}", id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_compare_migrations_and_log_no_pending() {
        // All migrations are in the database
        let files = vec![
            (1234567890, PathBuf::from("1234567890_create_users.up.sql")),
            (1234567891, PathBuf::from("1234567891_add_index.up.sql")),
        ];

        let migrations = vec![
            "1234567890".to_string(),
            "1234567891".to_string(),
        ];

        // This should not log any pending migrations
        compare_migrations_and_log(files, migrations, false);
        compare_migrations_and_log(vec![], vec![], true);
    }

    #[test]
    fn test_compare_migrations_and_log_with_pending() {
        let tmp_dir = tempdir().unwrap();

        // Create a real file for the test
        let file_path = tmp_dir.path().join("1234567892_new_migration.up.sql");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"CREATE TABLE new_table;").unwrap();

        let files = vec![
            (1234567890, PathBuf::from("1234567890_create_users.up.sql")),
            (1234567891, PathBuf::from("1234567891_add_index.up.sql")),
            (1234567892, file_path), // This one is pending
        ];

        let migrations = vec![
            "1234567890".to_string(),
            "1234567891".to_string(),
            // Missing 1234567892
        ];

        // This should log the pending migration
        compare_migrations_and_log(files, migrations, false);
    }

    #[test]
    fn test_compare_migrations_and_log_verbose_mode() {
        let tmp_dir = tempdir().unwrap();

        // Create a real file for verbose test
        let file_path = tmp_dir.path().join("1234567893_verbose_test.up.sql");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(b"CREATE TABLE verbose_test;\nALTER TABLE users ADD COLUMN email VARCHAR(255);").unwrap();

        let files = vec![
            (1234567893, file_path),
        ];

        let migrations = vec![]; // No migrations in database

        // This should log the pending migration with content in verbose mode
        compare_migrations_and_log(files, migrations, true);
    }

    #[test]
    fn test_compare_migrations_and_log_empty_files() {
        let files = vec![];
        let migrations = vec!["1234567890".to_string()];

        // Should handle empty files list gracefully
        compare_migrations_and_log(files, migrations, false);
    }

    #[test]
    fn test_compare_migrations_and_log_empty_migrations() {
        let files = vec![
            (1234567890, PathBuf::from("1234567890_test.up.sql")),
        ];
        let migrations = vec![];

        // All files should be considered pending
        compare_migrations_and_log(files, migrations, false);
    }

    #[test]
    fn test_compare_migrations_and_log_partial_match() {
        let tmp_dir = tempdir().unwrap();

        let file_path1 = tmp_dir.path().join("1234567890_applied.up.sql");
        let file_path2 = tmp_dir.path().join("1234567891_pending.up.sql");

        File::create(&file_path1).unwrap();
        File::create(&file_path2).unwrap();

        let files = vec![
            (1234567890, file_path1), // This is applied
            (1234567891, file_path2), // This is pending
        ];

        let migrations = vec![
            "1234567890".to_string(), // Only this one is in database
        ];

        // Should only log 1234567891 as pending
        compare_migrations_and_log(files, migrations, false);
    }
}
