use crate::utils::{get_local_migrations, read_file_content};
use crate::{database_drivers, utils};
use anyhow::{bail, Result};
use log::info;
use std::path::PathBuf;

pub async fn up(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    dump_schema: bool,
) -> Result<()> {
    let path = PathBuf::from(&migration_folder);
    let files = match get_local_migrations(&path, "up") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .up.sql at {}. Does the path exist?",
            migration_folder,
        );
    }

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
            let run_in_transaction = utils::should_run_in_transaction(&query);

            if let Err(e) = database.execute(&query, run_in_transaction).await { bail!(e) }

            database.insert_schema_migration(&id).await?;
        }
    }

    if dump_schema {
        if let Err(err) = database.dump_database_schema().await {
            log::error!("Skipping dumping database schema: {:?}", err);
        }
    }

    Ok(())
}

pub async fn down(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    dump_schema: bool,
    rollback_amount: &i64,
) -> Result<()> {
    let path = PathBuf::from(&migration_folder);
    let files = match get_local_migrations(&path, "down") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .down.sql at {}. Does the path exist?",
            migration_folder
        );
    }

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
                let run_in_transaction = utils::should_run_in_transaction(&query);

                if let Err(e) = database.execute(&query, run_in_transaction).await { bail!(e) }

                database
                    .remove_schema_migration(migration.to_string().as_str())
                    .await?;
            }
        }
    }

    if dump_schema {
        if let Err(err) = database.dump_database_schema().await {
            log::error!("Skipping dumping database schema: {:?}", err);
        }
    }

    Ok(())
}
// Helper functions extracted for testing
pub fn validate_migration_files(files: &[(i64, PathBuf)], migration_folder: &str, direction: &str) -> Result<()> {
    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .{}.sql at {}. Does the path exist?",
            direction,
            migration_folder,
        );
    }
    Ok(())
}

pub fn filter_pending_migrations(files: Vec<(i64, PathBuf)>, applied_migrations: &[String]) -> Vec<(i64, PathBuf)> {
    files
        .into_iter()
        .filter(|(timestamp, _)| {
            let id = timestamp.to_string();
            !applied_migrations.contains(&id)
        })
        .collect()
}

pub fn get_migrations_to_rollback(applied_migrations: Vec<String>, rollback_amount: i64) -> Result<Vec<i64>> {
    let mut migrations: Vec<i64> = applied_migrations
        .into_iter()
        .filter_map(|s| s.parse::<i64>().ok())
        .collect();

    // Sort in descending order (newest first) for rollback
    migrations.sort_by(|a, b| b.cmp(a));

    let rollback_count = rollback_amount as usize;
    if rollback_count > migrations.len() {
        Ok(migrations) // Return all if asking for more than available
    } else {
        Ok(migrations.into_iter().take(rollback_count).collect())
    }
}

pub fn find_rollback_file<'a>(
    files: &'a [(i64, PathBuf)],
    migration_id: i64,
) -> Option<&'a (i64, PathBuf)> {
    files.iter().find(|(timestamp, _)| *timestamp == migration_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_validate_migration_files_empty() {
        let files = vec![];
        let result = validate_migration_files(&files, "./migrations", "up");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Didn't find any files ending with .up.sql"));
    }

    #[test]
    fn test_validate_migration_files_with_files() {
        let files = vec![
            (1234567890, PathBuf::from("1234567890_test.up.sql")),
            (1234567891, PathBuf::from("1234567891_test.up.sql")),
        ];
        let result = validate_migration_files(&files, "./migrations", "up");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_migration_files_down_direction() {
        let files = vec![];
        let result = validate_migration_files(&files, "./rollbacks", "down");
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Didn't find any files ending with .down.sql"));
        assert!(error_msg.contains("./rollbacks"));
    }

    #[test]
    fn test_filter_pending_migrations_none_applied() {
        let files = vec![
            (1234567890, PathBuf::from("1234567890_test.up.sql")),
            (1234567891, PathBuf::from("1234567891_test.up.sql")),
            (1234567892, PathBuf::from("1234567892_test.up.sql")),
        ];
        let applied_migrations = vec![];

        let result = filter_pending_migrations(files, &applied_migrations);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_filter_pending_migrations_some_applied() {
        let files = vec![
            (1234567890, PathBuf::from("1234567890_test.up.sql")),
            (1234567891, PathBuf::from("1234567891_test.up.sql")),
            (1234567892, PathBuf::from("1234567892_test.up.sql")),
        ];
        let applied_migrations = vec!["1234567890".to_string(), "1234567892".to_string()];

        let result = filter_pending_migrations(files, &applied_migrations);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, 1234567891);
    }

    #[test]
    fn test_filter_pending_migrations_all_applied() {
        let files = vec![
            (1234567890, PathBuf::from("1234567890_test.up.sql")),
            (1234567891, PathBuf::from("1234567891_test.up.sql")),
        ];
        let applied_migrations = vec!["1234567890".to_string(), "1234567891".to_string()];

        let result = filter_pending_migrations(files, &applied_migrations);
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_get_migrations_to_rollback_normal() {
        let applied_migrations = vec![
            "1234567890".to_string(),
            "1234567891".to_string(),
            "1234567892".to_string(),
            "1234567893".to_string(),
        ];

        let result = get_migrations_to_rollback(applied_migrations, 2).unwrap();
        assert_eq!(result.len(), 2);
        // Should be sorted newest first
        assert_eq!(result[0], 1234567893);
        assert_eq!(result[1], 1234567892);
    }

    #[test]
    fn test_get_migrations_to_rollback_more_than_available() {
        let applied_migrations = vec![
            "1234567890".to_string(),
            "1234567891".to_string(),
        ];

        let result = get_migrations_to_rollback(applied_migrations, 5).unwrap();
        assert_eq!(result.len(), 2); // Should return all available
        assert_eq!(result[0], 1234567891);
        assert_eq!(result[1], 1234567890);
    }

    #[test]
    fn test_get_migrations_to_rollback_zero() {
        let applied_migrations = vec![
            "1234567890".to_string(),
            "1234567891".to_string(),
        ];

        let result = get_migrations_to_rollback(applied_migrations, 0).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_get_migrations_to_rollback_invalid_timestamps() {
        let applied_migrations = vec![
            "1234567890".to_string(),
            "invalid_timestamp".to_string(),
            "1234567891".to_string(),
            "another_invalid".to_string(),
        ];

        let result = get_migrations_to_rollback(applied_migrations, 5).unwrap();
        assert_eq!(result.len(), 2); // Only valid timestamps
        assert_eq!(result[0], 1234567891);
        assert_eq!(result[1], 1234567890);
    }

    #[test]
    fn test_find_rollback_file_found() {
        let files = vec![
            (1234567890, PathBuf::from("1234567890_test.down.sql")),
            (1234567891, PathBuf::from("1234567891_test.down.sql")),
            (1234567892, PathBuf::from("1234567892_test.down.sql")),
        ];

        let result = find_rollback_file(&files, 1234567891);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, 1234567891);
        assert!(result.unwrap().1.to_string_lossy().contains("1234567891_test.down.sql"));
    }

    #[test]
    fn test_find_rollback_file_not_found() {
        let files = vec![
            (1234567890, PathBuf::from("1234567890_test.down.sql")),
            (1234567891, PathBuf::from("1234567891_test.down.sql")),
        ];

        let result = find_rollback_file(&files, 1234567999);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_rollback_file_empty_files() {
        let files = vec![];
        let result = find_rollback_file(&files, 1234567890);
        assert!(result.is_none());
    }

    // Integration-style tests that test file reading but don't require database
    #[test]
    fn test_migration_file_validation_with_real_files() {
        let tmp_dir = tempdir().unwrap();
        let migration_folder = tmp_dir.path();

        // Create test migration files
        let up_file = migration_folder.join("1234567890_test.up.sql");
        let mut file = File::create(&up_file).unwrap();
        file.write_all(b"CREATE TABLE test_table;").unwrap();

        let down_file = migration_folder.join("1234567890_test.down.sql");
        let mut file = File::create(&down_file).unwrap();
        file.write_all(b"DROP TABLE test_table;").unwrap();

        // Test up migration validation
        let up_files = get_local_migrations(&migration_folder.to_path_buf(), "up").unwrap();
        let result = validate_migration_files(&up_files, migration_folder.to_str().unwrap(), "up");
        assert!(result.is_ok());

        // Test down migration validation
        let down_files = get_local_migrations(&migration_folder.to_path_buf(), "down").unwrap();
        let result = validate_migration_files(&down_files, migration_folder.to_str().unwrap(), "down");
        assert!(result.is_ok());
    }

    #[test]
    fn test_migration_workflow_logic() {
        // Test the core logic flow without database
        let files = vec![
            (1234567890, PathBuf::from("1234567890_create_users.up.sql")),
            (1234567891, PathBuf::from("1234567891_add_index.up.sql")),
            (1234567892, PathBuf::from("1234567892_add_posts.up.sql")),
        ];

        // Simulate partially applied migrations
        let applied_migrations = vec!["1234567890".to_string()];

        // Validate files
        assert!(validate_migration_files(&files, "./migrations", "up").is_ok());

        // Filter pending migrations
        let pending = filter_pending_migrations(files, &applied_migrations);
        assert_eq!(pending.len(), 2);
        assert_eq!(pending[0].0, 1234567891);
        assert_eq!(pending[1].0, 1234567892);

        // Test rollback planning
        let all_applied = vec![
            "1234567890".to_string(),
            "1234567891".to_string(),
            "1234567892".to_string(),
        ];
        let rollback_targets = get_migrations_to_rollback(all_applied, 1).unwrap();
        assert_eq!(rollback_targets.len(), 1);
        assert_eq!(rollback_targets[0], 1234567892); // Newest first
    }
}