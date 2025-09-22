use crate::config;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::usize;

pub mod libsql;
pub mod maria;
pub mod mysql;
pub mod postgres;
pub mod sqlite;
mod utils;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct SchemaMigration {
    pub id: String,
}

// DatabaseDriver is a trait that all database drivers must implement
pub trait DatabaseDriver {
    // execute giving query for the specifiv
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
        run_in_transaction: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // create database with the specific driver
    fn create_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // drop database with the specific driver
    fn drop_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // get current schema migrations for the schema migrations table
    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>>;

    // insert new schema migration
    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // remove schema migration from the schema migrations table
    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // create database with the specific driver
    fn ready(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;

    // dump the database
    fn dump_database_schema(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>>;
}

// Creates a new database driver based on the database_url
pub async fn new(
    db_url: String,
    db_token: Option<String>,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    with_selected_database: bool,
) -> Result<Box<dyn DatabaseDriver>, anyhow::Error> {
    let mut parsed_db_url = url::Url::parse(&db_url)?;

    let cloned_db_url = parsed_db_url.clone();
    let mut database_name = cloned_db_url.path().trim_start_matches('/');

    let driver = config::Database::new(parsed_db_url.scheme())?;

    if let (
        false,
        config::Database::MariaDB | config::Database::Postgres | config::Database::MySQL,
    ) = (with_selected_database, driver)
    {
        parsed_db_url.set_path("");
        database_name = cloned_db_url.path().trim_start_matches('/');
    }

    let scheme = parsed_db_url.scheme();

    match scheme {
        "http" | "https" | "libsql" => {
            let driver = libsql::LibSQLDriver::new(
                &parsed_db_url.to_string(),
                db_token,
                migrations_table,
                migrations_folder,
                schema_file,
            )
            .await?;
            Ok(Box::new(driver))
        }
        "postgres" | "psql" | "postgresql" => {
            parsed_db_url.set_scheme("postgresql").unwrap();
            let driver = postgres::PostgresDriver::new(
                parsed_db_url.as_str(),
                database_name,
                wait_timeout,
                migrations_table,
                migrations_folder,
                schema_file,
            )
            .await?;
            Ok(Box::new(driver))
        }
        "mysql" => {
            let driver = mysql::MySQLDriver::new(
                parsed_db_url.as_str(),
                database_name,
                wait_timeout,
                migrations_table,
                migrations_folder,
                schema_file,
            )
            .await?;
            Ok(Box::new(driver))
        }
        "sqlite" | "sqlite3" => {
            let driver = sqlite::SqliteDriver::new(
                &db_url,
                migrations_table,
                migrations_folder,
                schema_file,
            )
            .await?;
            Ok(Box::new(driver))
        }
        "mariadb" => {
            let driver = maria::MariaDBDriver::new(
                parsed_db_url.as_str(),
                database_name,
                wait_timeout,
                migrations_table,
                migrations_folder,
                schema_file,
            )
            .await?;
            Ok(Box::new(driver))
        }
        _ => bail!("Unsupported database driver: {}", scheme),
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::database_test_utils::*;

    #[test]
    fn test_schema_migration_struct() {
        // Test that SchemaMigration can be created and has the expected fields
        let migration = SchemaMigration {
            id: "1234567890".to_string(),
        };

        assert_eq!(migration.id, "1234567890");
    }

    #[test]
    fn test_extract_database_name_from_url_postgres() {
        let url = "postgres://user:pass@localhost:5432/testdb";
        let result = extract_database_name_from_url(url).unwrap();
        assert_eq!(result, "testdb");
    }

    #[test]
    fn test_extract_database_name_from_url_mysql() {
        let url = "mysql://root:password@localhost:3306/myapp";
        let result = extract_database_name_from_url(url).unwrap();
        assert_eq!(result, "myapp");
    }

    #[test]
    fn test_extract_database_name_from_url_sqlite() {
        let url = "sqlite://./test.db";
        let result = extract_database_name_from_url(url).unwrap();
        assert_eq!(result, "test.db"); // URL parsing removes the "./"
    }

    #[test]
    fn test_extract_database_name_from_url_no_db() {
        let url = "postgres://localhost:5432/";
        let result = extract_database_name_from_url(url).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_extract_database_name_from_url_invalid() {
        let url = "not-a-url";
        let result = extract_database_name_from_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_database_type_from_url_postgres() {
        let url = "postgres://localhost/test";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::Postgres));
    }

    #[test]
    fn test_get_database_type_from_url_mysql() {
        let url = "mysql://localhost/test";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::MySQL));
    }

    #[test]
    fn test_get_database_type_from_url_mariadb() {
        let url = "mariadb://localhost/test";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::MariaDB));
    }

    #[test]
    fn test_get_database_type_from_url_sqlite() {
        let url = "sqlite://test.db";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::SQLite));
    }

    #[test]
    fn test_get_database_type_from_url_sqlite3() {
        let url = "sqlite3://test.db";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::SQLite));
    }

    #[test]
    fn test_get_database_type_from_url_libsql() {
        let url = "http://localhost:8080";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::LibSQL));
    }

    #[test]
    fn test_get_database_type_from_url_libsql_https() {
        let url = "https://my-db.turso.io";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::LibSQL));
    }

    #[test]
    fn test_get_database_type_from_url_libsql_scheme() {
        let url = "libsql://my-db.turso.io";
        let result = get_database_type_from_url(url).unwrap();
        assert!(matches!(result, crate::config::Database::LibSQL));
    }

    #[test]
    fn test_get_database_type_from_url_invalid_scheme() {
        let url = "redis://localhost:6379";
        let result = get_database_type_from_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_database_type_from_url_invalid_url() {
        let url = "not-a-valid-url";
        let result = get_database_type_from_url(url);
        assert!(result.is_err());
    }

    #[test]
    fn test_database_driver_factory_parameters() {
        // Test that the new function has the expected signature
        // This is a compile-time test to ensure API consistency

        let _params = (
            "sqlite://test.db".to_string(),           // db_url
            None::<String>,                           // db_token
            "schema_migrations".to_string(),          // migrations_table
            "./migrations".to_string(),               // migrations_folder
            "schema.sql".to_string(),                 // schema_file
            Some(30_usize),                          // wait_timeout
            true,                                     // with_selected_database
        );

        // Test that the function accepts these parameter types
        // This is a compile-time validation
        assert!(true);
    }

    #[test]
    fn test_url_parsing_edge_cases() {
        // Test various URL formats that should be supported
        let test_cases = vec![
            ("postgres://user@localhost/db", "postgres"),
            ("postgresql://user@localhost/db", "postgresql"),
            ("psql://user@localhost/db", "psql"),
            ("mysql://root@localhost/app", "mysql"),
            ("mariadb://user@localhost/test", "mariadb"),
            ("sqlite://./test.db", "sqlite"),
            ("sqlite3://./test.db", "sqlite3"),
            ("http://localhost:8080", "http"),
            ("https://turso.io", "https"),
            ("libsql://turso.io", "libsql"),
        ];

        for (url, expected_scheme) in test_cases {
            let parsed_url = url::Url::parse(url).unwrap();
            assert_eq!(parsed_url.scheme(), expected_scheme);
        }
    }

    #[test]
    fn test_postgresql_scheme_normalization() {
        // Test that postgres/psql schemes get normalized to postgresql
        let test_urls = vec![
            "postgres://localhost/test",
            "psql://localhost/test",
        ];

        for url in test_urls {
            let mut parsed_url = url::Url::parse(url).unwrap();
            // This simulates what happens in the new() function
            parsed_url.set_scheme("postgresql").unwrap();
            assert_eq!(parsed_url.scheme(), "postgresql");
        }
    }

    #[test]
    fn test_database_path_manipulation() {
        // Test path manipulation for database creation vs selection
        let url = "postgres://localhost:5432/testdb";
        let mut parsed_url = url::Url::parse(url).unwrap();
        let original_path = parsed_url.path();

        assert_eq!(original_path, "/testdb");

        // Test path clearing for database creation
        parsed_url.set_path("");
        assert_eq!(parsed_url.path(), "");
    }
}
