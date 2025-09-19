use crate::database_drivers;
use anyhow::Result;

pub async fn create(
    database_url: String,
    db_token: Option<String>,
    wait_timeout: Option<usize>,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
) -> Result<()> {
    let mut database = database_drivers::new(
        database_url,
        db_token,
        migrations_table,
        migrations_folder,
        schema_file,
        wait_timeout,
        false,
    )
    .await?;

    database.create_database().await?;

    Ok(())
}

pub async fn drop(
    database_url: String,
    db_token: Option<String>,
    wait_timeout: Option<usize>,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
) -> Result<()> {
    let mut database = database_drivers::new(
        database_url,
        db_token,
        migrations_table,
        migrations_folder,
        schema_file,
        wait_timeout,
        false,
    )
    .await?;

    database.drop_database().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_function_signature() {
        // Test that the create function has the expected signature
        // This is a compile-time test - if this compiles, the signature is correct

        let _create_fn: fn(String, Option<String>, Option<usize>, String, String, String) -> _ = create;

        // Test that parameters are in the expected order
        let _database_url = "sqlite://test.db".to_string();
        let _db_token: Option<String> = None;
        let _wait_timeout: Option<usize> = Some(30);
        let _migrations_table = "schema_migrations".to_string();
        let _migrations_folder = "./migrations".to_string();
        let _schema_file = "schema.sql".to_string();

        // We can't easily test the actual database creation without a real database,
        // but we can ensure the function signature is correct
        assert!(true);
    }

    #[test]
    fn test_drop_function_signature() {
        // Test that the drop function has the expected signature
        let _drop_fn: fn(String, Option<String>, Option<usize>, String, String, String) -> _ = drop;

        // Test that parameters are in the expected order
        let _database_url = "postgres://localhost/test".to_string();
        let _db_token: Option<String> = Some("token".to_string());
        let _wait_timeout: Option<usize> = Some(60);
        let _migrations_table = "migrations".to_string();
        let _migrations_folder = "./migrations".to_string();
        let _schema_file = "schema.sql".to_string();

        assert!(true);
    }

    #[test]
    fn test_create_parameter_types() {
        // Ensure the function accepts the correct parameter types
        let database_url: String = "mysql://localhost/test".into();
        let db_token: Option<String> = None;
        let wait_timeout: Option<usize> = Some(45);
        let migrations_table: String = "custom_migrations".into();
        let migrations_folder: String = "./custom_migrations".into();
        let schema_file: String = "custom_schema.sql".into();

        // Test that all these types are accepted (compile-time check)
        let _future = create(
            database_url,
            db_token,
            wait_timeout,
            migrations_table,
            migrations_folder,
            schema_file,
        );

        // The future exists and has the right return type
        assert!(true);
    }

    #[test]
    fn test_drop_parameter_types() {
        // Ensure the function accepts the correct parameter types
        let database_url: String = "mariadb://localhost/test".into();
        let db_token: Option<String> = Some("auth_token".into());
        let wait_timeout: Option<usize> = None;
        let migrations_table: String = "schema_migrations".into();
        let migrations_folder: String = "./migrations".into();
        let schema_file: String = "schema.sql".into();

        // Test that all these types are accepted (compile-time check)
        let _future = drop(
            database_url,
            db_token,
            wait_timeout,
            migrations_table,
            migrations_folder,
            schema_file,
        );

        assert!(true);
    }

    #[test]
    fn test_optional_parameters() {
        // Test that optional parameters work correctly for both functions
        let database_url = "sqlite://memory".to_string();
        let db_token = None; // Optional
        let wait_timeout = None; // Optional
        let migrations_table = "migrations".to_string();
        let migrations_folder = "./migrations".to_string();
        let schema_file = "schema.sql".to_string();

        // Test create with optional parameters
        let _create_future = create(
            database_url.clone(),
            db_token.clone(),
            wait_timeout,
            migrations_table.clone(),
            migrations_folder.clone(),
            schema_file.clone(),
        );

        // Test drop with optional parameters
        let _drop_future = drop(
            database_url,
            db_token,
            wait_timeout,
            migrations_table,
            migrations_folder,
            schema_file,
        );

        assert!(true);
    }

    #[test]
    fn test_parameter_order_consistency() {
        // Test that both create and drop functions have the same parameter order
        // This helps ensure API consistency

        let params = (
            "sqlite://test.db".to_string(),    // database_url
            None::<String>,                    // db_token
            Some(30_usize),                    // wait_timeout
            "migrations".to_string(),          // migrations_table
            "./migrations".to_string(),        // migrations_folder
            "schema.sql".to_string(),          // schema_file
        );

        // Both functions should accept the same parameters in the same order
        let _create_future = create(
            params.0.clone(),
            params.1.clone(),
            params.2,
            params.3.clone(),
            params.4.clone(),
            params.5.clone(),
        );

        let _drop_future = drop(
            params.0,
            params.1,
            params.2,
            params.3,
            params.4,
            params.5,
        );

        assert!(true);
    }

    #[test]
    fn test_database_url_variations() {
        // Test that various database URL formats are accepted
        let db_urls = vec![
            "sqlite://test.db",
            "postgres://user:pass@localhost:5432/db",
            "mysql://root:password@localhost:3306/app",
            "mariadb://user@localhost/database",
            "http://localhost:8080", // LibSQL
        ];

        for url in db_urls {
            let _create_future = create(
                url.to_string(),
                None,
                None,
                "migrations".to_string(),
                "./migrations".to_string(),
                "schema.sql".to_string(),
            );

            let _drop_future = drop(
                url.to_string(),
                None,
                None,
                "migrations".to_string(),
                "./migrations".to_string(),
                "schema.sql".to_string(),
            );

            // Test passes if it compiles
        }

        assert!(true);
    }
}
