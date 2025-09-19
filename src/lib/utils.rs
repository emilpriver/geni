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

pub fn should_run_in_transaction(query: &str) -> bool {
    let first_line = query.split_once('\n').unwrap_or(("", "")).0;

    if first_line.contains("transaction: no") {
        return false;
    }

    if first_line.contains("transaction:no") {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_without_transaction_no_in_first_line() {
        let query = "something else\ntransaction: no";
        assert!(should_run_in_transaction(query));
    }

    #[test]
    fn test_with_empty_line() {
        let query = "";
        assert!(should_run_in_transaction(query));
    }

    #[test]
    fn test_with_transaction_yes_in_first_line() {
        let query = "transaction: yes\nSELECT * FROM users";
        assert!(should_run_in_transaction(query));
    }

    #[test]
    fn test_with_transaction_no_in_first_line() {
        let query = "transaction: no\nSELECT * FROM users";
        assert!(!should_run_in_transaction(query));
    }

    #[test]
    fn test_with_transaction_no_in_first_line_without_space() {
        let query = "transaction:no\nSELECT * FROM users";
        assert!(!should_run_in_transaction(query));
    }

    #[test]
    fn test_get_local_migrations_with_valid_files() {
        let tmp_dir = tempdir().unwrap();
        let migration_folder = tmp_dir.path();

        // Create test migration files with timestamps
        let files = vec![
            ("1234567890_create_users.up.sql", "CREATE TABLE users;"),
            ("1234567891_add_index.up.sql", "CREATE INDEX;"),
            ("1234567892_drop_table.up.sql", "DROP TABLE;"),
        ];

        for (filename, content) in &files {
            let file_path = migration_folder.join(filename);
            let mut file = File::create(&file_path).unwrap();
            file.write_all(content.as_bytes()).unwrap();
        }

        let result = get_local_migrations(&migration_folder.to_path_buf(), "up").unwrap();

        assert_eq!(result.len(), 3);
        // Should be sorted by timestamp
        assert_eq!(result[0].0, 1234567890);
        assert_eq!(result[1].0, 1234567891);
        assert_eq!(result[2].0, 1234567892);

        // Check filenames
        assert!(result[0].1.to_string_lossy().contains("create_users"));
        assert!(result[1].1.to_string_lossy().contains("add_index"));
        assert!(result[2].1.to_string_lossy().contains("drop_table"));
    }

    #[test]
    fn test_get_local_migrations_empty_directory() {
        let tmp_dir = tempdir().unwrap();
        let migration_folder = tmp_dir.path();

        let result = get_local_migrations(&migration_folder.to_path_buf(), "up").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_get_local_migrations_filters_by_ending() {
        let tmp_dir = tempdir().unwrap();
        let migration_folder = tmp_dir.path();

        // Create mixed migration files
        let files = vec![
            "1234567890_test.up.sql",
            "1234567890_test.down.sql",
            "1234567891_other.up.sql",
            "1234567892_wrong.txt", // Wrong extension
        ];

        for filename in &files {
            let file_path = migration_folder.join(filename);
            File::create(&file_path).unwrap();
        }

        // Test filtering for "up" files
        let up_result = get_local_migrations(&migration_folder.to_path_buf(), "up").unwrap();
        assert_eq!(up_result.len(), 2);

        // Test filtering for "down" files
        let down_result = get_local_migrations(&migration_folder.to_path_buf(), "down").unwrap();
        assert_eq!(down_result.len(), 1);
    }

    #[test]
    fn test_get_local_migrations_nonexistent_directory() {
        let nonexistent_path = PathBuf::from("/this/path/does/not/exist");
        let result = get_local_migrations(&nonexistent_path, "up");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_local_migrations_sorting() {
        let tmp_dir = tempdir().unwrap();
        let migration_folder = tmp_dir.path();

        // Create files with timestamps out of order
        let files = vec![
            "1234567892_latest.up.sql",
            "1234567890_oldest.up.sql",
            "1234567891_middle.up.sql",
        ];

        for filename in &files {
            let file_path = migration_folder.join(filename);
            File::create(&file_path).unwrap();
        }

        let result = get_local_migrations(&migration_folder.to_path_buf(), "up").unwrap();

        // Should be sorted by timestamp ascending
        assert_eq!(result[0].0, 1234567890);
        assert_eq!(result[1].0, 1234567891);
        assert_eq!(result[2].0, 1234567892);
    }

    #[test]
    fn test_read_file_content() {
        let tmp_dir = tempdir().unwrap();
        let file_path = tmp_dir.path().join("test.sql");
        let content = "SELECT * FROM users;\nCREATE TABLE posts;";

        let mut file = File::create(&file_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let result = read_file_content(&file_path);
        assert_eq!(result, content);
    }

    #[test]
    fn test_read_file_content_empty_file() {
        let tmp_dir = tempdir().unwrap();
        let file_path = tmp_dir.path().join("empty.sql");

        File::create(&file_path).unwrap();

        let result = read_file_content(&file_path);
        assert_eq!(result, "");
    }

    #[test]
    #[should_panic]
    fn test_read_file_content_nonexistent_file() {
        let nonexistent_path = PathBuf::from("/this/file/does/not/exist.sql");
        read_file_content(&nonexistent_path);
    }
}
