use anyhow::{bail, Result};
use std::path::PathBuf;

/// Helper function to validate migration files for testing
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

/// Helper function to filter pending migrations for testing
pub fn filter_pending_migrations(files: Vec<(i64, PathBuf)>, applied_migrations: &[String]) -> Vec<(i64, PathBuf)> {
    files
        .into_iter()
        .filter(|(timestamp, _)| {
            let id = timestamp.to_string();
            !applied_migrations.contains(&id)
        })
        .collect()
}

/// Helper function to get migrations to rollback for testing
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

/// Helper function to find rollback file for testing
pub fn find_rollback_file<'a>(
    files: &'a [(i64, PathBuf)],
    migration_id: i64,
) -> Option<&'a (i64, PathBuf)> {
    files.iter().find(|(timestamp, _)| *timestamp == migration_id)
}