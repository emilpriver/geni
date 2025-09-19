use anyhow::Result;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub async fn write_to_schema_file(
    content: String,
    migrations_folder: String,
    schema_file: String,
) -> Result<()> {
    let schema_path = format!("{}/{}", migrations_folder, schema_file);
    let path = Path::new(schema_path.as_str());

    if File::open(path.to_str().unwrap()).is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        File::create(&schema_path)?;
    };

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(schema_path.as_str())?;

    file.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_write_to_schema_file_new_file() {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "test_schema.sql".to_string();
        let content = "CREATE TABLE users (id INTEGER PRIMARY KEY);".to_string();

        let result = write_to_schema_file(content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result.is_ok());

        // Verify file was created and has correct content
        let schema_path = format!("{}/{}", migrations_folder, schema_file);
        let file_content = std::fs::read_to_string(&schema_path).unwrap();
        assert_eq!(file_content, content);
    }

    #[tokio::test]
    async fn test_write_to_schema_file_overwrite() {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "overwrite_schema.sql".to_string();

        // Write initial content
        let initial_content = "CREATE TABLE test1 (id INTEGER);".to_string();
        let result1 = write_to_schema_file(initial_content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result1.is_ok());

        // Write new content (should overwrite)
        let new_content = "CREATE TABLE test2 (id INTEGER, name TEXT);".to_string();
        let result2 = write_to_schema_file(new_content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result2.is_ok());

        // Verify file was overwritten
        let schema_path = format!("{}/{}", migrations_folder, schema_file);
        let file_content = std::fs::read_to_string(&schema_path).unwrap();
        assert_eq!(file_content, new_content);
        assert_ne!(file_content, initial_content);
    }

    #[tokio::test]
    async fn test_write_to_schema_file_nested_directory() {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().join("nested").join("path").to_str().unwrap().to_string();
        let schema_file = "nested_schema.sql".to_string();
        let content = "CREATE INDEX idx_users_email ON users(email);".to_string();

        let result = write_to_schema_file(content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result.is_ok());

        // Verify nested directories were created
        let schema_path = format!("{}/{}", migrations_folder, schema_file);
        assert!(Path::new(&schema_path).exists());

        let file_content = std::fs::read_to_string(&schema_path).unwrap();
        assert_eq!(file_content, content);
    }

    #[tokio::test]
    async fn test_write_to_schema_file_empty_content() {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "empty_schema.sql".to_string();
        let content = "".to_string();

        let result = write_to_schema_file(content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result.is_ok());

        let schema_path = format!("{}/{}", migrations_folder, schema_file);
        let file_content = std::fs::read_to_string(&schema_path).unwrap();
        assert_eq!(file_content, "");
    }

    #[tokio::test]
    async fn test_write_to_schema_file_large_content() {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "large_schema.sql".to_string();

        // Create large content
        let mut content = String::new();
        for i in 0..1000 {
            content.push_str(&format!("CREATE TABLE table_{} (id INTEGER PRIMARY KEY);\n", i));
        }

        let result = write_to_schema_file(content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result.is_ok());

        let schema_path = format!("{}/{}", migrations_folder, schema_file);
        let file_content = std::fs::read_to_string(&schema_path).unwrap();
        assert_eq!(file_content, content);
        assert!(file_content.len() > 10000); // Verify it's actually large
    }

    #[tokio::test]
    async fn test_write_to_schema_file_special_characters() {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "special_schema.sql".to_string();
        let content = "CREATE TABLE test (id INTEGER, data TEXT DEFAULT 'special chars: éñ中文🚀');".to_string();

        let result = write_to_schema_file(content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result.is_ok());

        let schema_path = format!("{}/{}", migrations_folder, schema_file);
        let file_content = std::fs::read_to_string(&schema_path).unwrap();
        assert_eq!(file_content, content);
    }

    #[tokio::test]
    async fn test_write_to_schema_file_path_formatting() {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "path_test.sql".to_string();
        let content = "SELECT 1;".to_string();

        let result = write_to_schema_file(content.clone(), migrations_folder.clone(), schema_file.clone()).await;
        assert!(result.is_ok());

        // Test that the path is formatted correctly
        let expected_path = format!("{}/{}", migrations_folder, schema_file);
        assert!(Path::new(&expected_path).exists());

        // Test with different path separators
        let schema_file_with_subdir = "subdir/schema.sql".to_string();
        let result2 = write_to_schema_file(content.clone(), migrations_folder.clone(), schema_file_with_subdir.clone()).await;
        assert!(result2.is_ok());

        let expected_path2 = format!("{}/{}", migrations_folder, schema_file_with_subdir);
        assert!(Path::new(&expected_path2).exists());
    }
}
