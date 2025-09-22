use crate::database_drivers;
use anyhow::Result;

pub async fn dump(
    database_url: String,
    database_token: Option<String>,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
) -> Result<()> {
    let mut database = database_drivers::new(
        database_url,
        database_token,
        migrations_table,
        migrations_folder,
        schema_file,
        wait_timeout,
        true,
    )
    .await?;

    database.dump_database_schema().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_dump_function_signature() {
        // Test that the dump function has the expected signature
        // This is a compile-time test - if this compiles, the signature is correct

        let _dump_fn: fn(String, Option<String>, String, String, String, Option<usize>) -> _ = dump;

        // Test that parameters are in the expected order
        let _database_url = "sqlite://test.db".to_string();
        let _database_token: Option<String> = None;
        let _migrations_table = "migrations".to_string();
        let _migrations_folder = "./migrations".to_string();
        let _schema_file = "schema.sql".to_string();
        let _wait_timeout: Option<usize> = Some(30);

        // We can't easily test the actual dump functionality without a real database,
        // but we can ensure the function signature is correct
        assert!(true); // This test mainly serves as documentation
    }

    #[test]
    fn test_dump_parameter_types() {
        // Ensure the function accepts the correct parameter types
        let database_url: String = "postgres://localhost/test".into();
        let database_token: Option<String> = Some("token".into());
        let migrations_table: String = "schema_migrations".into();
        let migrations_folder: String = "./migrations".into();
        let schema_file: String = "schema.sql".into();
        let wait_timeout: Option<usize> = Some(60);

        // Test that all these types are accepted (compile-time check)
        let _future = dump(
            database_url,
            database_token,
            migrations_table,
            migrations_folder,
            schema_file,
            wait_timeout,
        );

        // The future exists and has the right return type
        assert!(true);
    }

    #[test]
    fn test_dump_optional_parameters() {
        // Test that optional parameters work correctly
        let database_url = "sqlite://memory".to_string();
        let database_token = None; // Optional
        let migrations_table = "migrations".to_string();
        let migrations_folder = "./migrations".to_string();
        let schema_file = "schema.sql".to_string();
        let wait_timeout = None; // Optional

        let _future = dump(
            database_url,
            database_token,
            migrations_table,
            migrations_folder,
            schema_file,
            wait_timeout,
        );

        assert!(true);
    }

    async fn setup_test_database_with_schema(database_url: &str) -> anyhow::Result<()> {
        use crate::database_drivers;

        let mut client = database_drivers::new(
            database_url.to_string(),
            None,
            "schema_migrations".to_string(),
            "./migrations".to_string(),
            "schema.sql".to_string(),
            Some(30),
            true,
        )
        .await?;

        // Create some test tables to dump
        let test_queries = vec![
            "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE);",
            "CREATE TABLE test_posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, content TEXT, user_id INTEGER);",
            "CREATE INDEX idx_posts_user_id ON test_posts(user_id);",
        ];

        for query in test_queries {
            // Try to execute the query, but don't fail if table already exists
            let _ = client.execute(query, false).await;
        }

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_dump_sqlite() -> anyhow::Result<()> {
        let tmp_dir = tempdir().unwrap();
        let db_file = tmp_dir.path().join("test_dump.sqlite");
        let schema_file = "sqlite_dump_test_schema.sql";
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();

        // Create the database file
        File::create(&db_file)?;

        let database_url = format!("sqlite://{}", db_file.to_str().unwrap());

        // Setup test schema
        setup_test_database_with_schema(&database_url).await?;

        // Test the dump function
        let result = dump(
            database_url,
            None,
            "schema_migrations".to_string(),
            migrations_folder.clone(),
            schema_file.to_string(),
            Some(30),
        )
        .await;

        assert!(result.is_ok(), "Dump should succeed for SQLite");

        // Check that schema file was created
        let schema_path = Path::new(&migrations_folder).join(schema_file);
        assert!(schema_path.exists(), "Schema file should be created");

        // Verify schema file has content
        let schema_content = fs::read_to_string(&schema_path)?;
        assert!(!schema_content.trim().is_empty(), "Schema file should not be empty");

        // Check for our test tables in the schema
        assert!(schema_content.contains("test_users"), "Schema should contain the 'test_users' table definition");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_dump_postgres() -> anyhow::Result<()> {
        env::set_var("DATABASE_SCHEMA_FILE", "postgres_dump_test_schema.sql");
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "postgres_dump_test_schema.sql";

        let database_url = "psql://postgres:mysecretpassword@localhost:6437/app?sslmode=disable";

        // Setup test schema
        if let Err(e) = setup_test_database_with_schema(database_url).await {
            // Skip test if database is not available
            log::info!("Skipping Postgres dump test - database not available: {}", e);
            return Ok(());
        }

        // Test the dump function
        let result = dump(
            database_url.to_string(),
            None,
            "schema_migrations".to_string(),
            migrations_folder.clone(),
            schema_file.to_string(),
            Some(30),
        )
        .await;

        assert!(result.is_ok(), "Dump should succeed for Postgres");

        // Check that schema file was created
        let schema_path = Path::new(&migrations_folder).join(schema_file);
        assert!(schema_path.exists(), "Schema file should be created");

        // Verify schema file has content
        let schema_content = fs::read_to_string(&schema_path)?;
        assert!(!schema_content.trim().is_empty(), "Schema file should not be empty");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_dump_mysql() -> anyhow::Result<()> {
        env::set_var("DATABASE_SCHEMA_FILE", "mysql_dump_test_schema.sql");
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "mysql_dump_test_schema.sql";

        let database_url = "mysql://root:password@localhost:3306/app";

        // Setup test schema
        if let Err(_) = setup_test_database_with_schema(database_url).await {
            // Skip test if database is not available
            println!("Skipping MySQL dump test - database not available");
            return Ok(());
        }

        // Test the dump function
        let result = dump(
            database_url.to_string(),
            None,
            "schema_migrations".to_string(),
            migrations_folder.clone(),
            schema_file.to_string(),
            Some(30),
        )
        .await;

        assert!(result.is_ok(), "Dump should succeed for MySQL");

        // Check that schema file was created
        let schema_path = Path::new(&migrations_folder).join(schema_file);
        assert!(schema_path.exists(), "Schema file should be created");

        // Verify schema file has content
        let schema_content = fs::read_to_string(&schema_path)?;
        assert!(!schema_content.trim().is_empty(), "Schema file should not be empty");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_dump_mariadb() -> anyhow::Result<()> {
        env::set_var("DATABASE_SCHEMA_FILE", "mariadb_dump_test_schema.sql");
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "mariadb_dump_test_schema.sql";

        let database_url = "mariadb://root:password@localhost:3307/app";

        // Setup test schema
        if let Err(_) = setup_test_database_with_schema(database_url).await {
            // Skip test if database is not available
            println!("Skipping MariaDB dump test - database not available");
            return Ok(());
        }

        // Test the dump function
        let result = dump(
            database_url.to_string(),
            None,
            "schema_migrations".to_string(),
            migrations_folder.clone(),
            schema_file.to_string(),
            Some(30),
        )
        .await;

        assert!(result.is_ok(), "Dump should succeed for MariaDB");

        // Check that schema file was created
        let schema_path = Path::new(&migrations_folder).join(schema_file);
        assert!(schema_path.exists(), "Schema file should be created");

        // Verify schema file has content
        let schema_content = fs::read_to_string(&schema_path)?;
        assert!(!schema_content.trim().is_empty(), "Schema file should not be empty");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_dump_libsql() -> anyhow::Result<()> {
        env::set_var("DATABASE_SCHEMA_FILE", "libsql_dump_test_schema.sql");
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "libsql_dump_test_schema.sql";

        let database_url = "http://localhost:6000";

        // Setup test schema
        if let Err(_) = setup_test_database_with_schema(database_url).await {
            // Skip test if database is not available
            println!("Skipping LibSQL dump test - database not available");
            return Ok(());
        }

        // Test the dump function
        let result = dump(
            database_url.to_string(),
            None,
            "schema_migrations".to_string(),
            migrations_folder.clone(),
            schema_file.to_string(),
            Some(30),
            ).await;

        assert!(result.is_ok(), "Dump should succeed for LibSQL");

        // Check that schema file was created
        let schema_path = Path::new(&migrations_folder).join(schema_file);
        assert!(schema_path.exists(), "Schema file should be created");

        // Verify schema file has content
        let schema_content = fs::read_to_string(&schema_path)?;
        assert!(!schema_content.trim().is_empty(), "Schema file should not be empty");

        Ok(())
    }

    #[tokio::test]
    #[serial]
    async fn test_dump_with_invalid_database_url() -> anyhow::Result<()> {
        let tmp_dir = tempdir().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();

        let result = dump(
            "invalid://database/url".to_string(),
            None,
            "schema_migrations".to_string(),
            migrations_folder,
            "schema.sql".to_string(),
            Some(30),
        )
        .await;

        assert!(result.is_err(), "Dump should fail with invalid database URL");

        Ok(())
    }
}
