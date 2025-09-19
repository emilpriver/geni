use anyhow::{bail, Result};
use log::info;

/// Helper function to extract database name from URL for testing
pub fn extract_database_name_from_url(url: &str) -> Result<String> {
    let parsed_url = url::Url::parse(url)?;
    let database_name = parsed_url.path().trim_start_matches('/');
    Ok(database_name.to_string())
}

/// Helper function to determine database type from URL scheme for testing
pub fn get_database_type_from_url(url: &str) -> Result<crate::config::Database> {
    let parsed_url = url::Url::parse(url)?;
    crate::config::Database::new(parsed_url.scheme())
}

/// Helper function to parse SQLite path from URL for testing
pub fn parse_sqlite_path(db_url: &str) -> &str {
    if db_url.contains("://") {
        db_url.split_once("://").unwrap().1
    } else {
        db_url
    }
}

/// Helper function to validate LibSQL URLs for testing
pub fn validate_libsql_url(db_url: &str) -> Result<bool> {
    if db_url.starts_with("libsql://./") {
        bail!("libsql:// should only be used with remote database. Use sqlite:// protocol when running local sqlite files");
    }

    let valid_schemes = ["http://", "https://", "libsql://"];
    let is_valid = valid_schemes.iter().any(|scheme| db_url.starts_with(scheme));

    if !is_valid {
        bail!("Invalid LibSQL URL scheme. Must start with http://, https://, or libsql://");
    }

    Ok(true)
}

/// Helper function to get auth token or default for testing
pub fn get_auth_token_or_default(token: Option<String>) -> String {
    match token {
        Some(t) => t,
        None => {
            info!("Token is not set, using empty string");
            "".to_string()
        }
    }
}

/// Helper function to validate PostgreSQL connection URL for testing
pub fn validate_postgres_url(url: &str) -> Result<bool> {
    if !url.starts_with("postgres://") && !url.starts_with("postgresql://") && !url.starts_with("psql://") {
        bail!("Invalid PostgreSQL URL scheme. Must start with postgres://, postgresql://, or psql://");
    }
    Ok(true)
}

/// Helper function to generate PostgreSQL CREATE DATABASE query for testing
pub fn generate_postgres_create_db_query(db_name: &str) -> String {
    format!("CREATE DATABASE {}", db_name)
}

/// Helper function to generate PostgreSQL DROP DATABASE query for testing
pub fn generate_postgres_drop_db_query(db_name: &str) -> String {
    format!("DROP DATABASE {}", db_name)
}

/// Helper function to generate PostgreSQL migrations table query for testing
pub fn generate_postgres_migrations_table_query(table_name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) PRIMARY KEY)",
        table_name
    )
}

/// Helper function to generate PostgreSQL INSERT migration query for testing
pub fn generate_postgres_insert_migration_query(table_name: &str) -> String {
    format!("INSERT INTO {} (id) VALUES ($1)", table_name)
}

/// Helper function to generate PostgreSQL DELETE migration query for testing
pub fn generate_postgres_delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {} WHERE id = $1", table_name)
}

/// Helper function to generate PostgreSQL SELECT migrations query for testing
pub fn generate_postgres_select_migrations_query(table_name: &str) -> String {
    format!("SELECT id FROM {} ORDER BY id DESC", table_name)
}

/// Helper function to validate wait timeout for testing
pub fn validate_wait_timeout(timeout: Option<usize>) -> usize {
    timeout.unwrap_or(0)
}

/// Helper function to validate MySQL connection URL for testing
pub fn validate_mysql_url(url: &str) -> Result<bool> {
    if !url.starts_with("mysql://") {
        bail!("Invalid MySQL URL scheme. Must start with mysql://");
    }
    Ok(true)
}

/// Helper function to generate MySQL CREATE DATABASE query for testing
pub fn generate_mysql_create_db_query(db_name: &str) -> String {
    format!("CREATE DATABASE IF NOT EXISTS {}", db_name)
}

/// Helper function to generate MySQL DROP DATABASE query for testing
pub fn generate_mysql_drop_db_query(db_name: &str) -> String {
    format!("DROP DATABASE IF EXISTS {}", db_name)
}

/// Helper function to generate MySQL migrations table query for testing
pub fn generate_mysql_migrations_table_query(table_name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) PRIMARY KEY)",
        table_name
    )
}

/// Helper function to generate MySQL INSERT migration query for testing
pub fn generate_mysql_insert_migration_query(table_name: &str) -> String {
    format!("INSERT INTO {} (id) VALUES (?)", table_name)
}

/// Helper function to generate MySQL DELETE migration query for testing
pub fn generate_mysql_delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {} WHERE id = ?", table_name)
}

/// Helper function to generate MySQL SELECT migrations query for testing
pub fn generate_mysql_select_migrations_query(table_name: &str) -> String {
    format!("SELECT id FROM {} ORDER BY id DESC", table_name)
}

/// Helper function to normalize MySQL localhost to 127.0.0.1 for testing
pub fn normalize_mysql_localhost_url(url: &str) -> Result<String> {
    let mut parsed_url = url::Url::parse(url)?;
    if parsed_url.host_str() == Some("localhost") {
        parsed_url.set_host(Some("127.0.0.1"))?;
    }
    Ok(parsed_url.to_string())
}

/// Helper function to validate MariaDB connection URL for testing
pub fn validate_mariadb_url(url: &str) -> Result<bool> {
    if !url.starts_with("mariadb://") {
        bail!("Invalid MariaDB URL scheme. Must start with mariadb://");
    }
    Ok(true)
}

/// Helper function to generate MariaDB CREATE DATABASE query for testing
pub fn generate_mariadb_create_db_query(db_name: &str) -> String {
    format!("CREATE DATABASE IF NOT EXISTS {}", db_name)
}

/// Helper function to generate MariaDB DROP DATABASE query for testing
pub fn generate_mariadb_drop_db_query(db_name: &str) -> String {
    format!("DROP DATABASE IF EXISTS {}", db_name)
}

/// Helper function to generate MariaDB migrations table query for testing
pub fn generate_mariadb_migrations_table_query(table_name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) PRIMARY KEY)",
        table_name
    )
}

/// Helper function to generate MariaDB INSERT migration query for testing
pub fn generate_mariadb_insert_migration_query(table_name: &str) -> String {
    format!("INSERT INTO {} (id) VALUES (?)", table_name)
}

/// Helper function to generate MariaDB DELETE migration query for testing
pub fn generate_mariadb_delete_migration_query(table_name: &str) -> String {
    format!("DELETE FROM {} WHERE id = ?", table_name)
}

/// Helper function to generate MariaDB SELECT migrations query for testing
pub fn generate_mariadb_select_migrations_query(table_name: &str) -> String {
    format!("SELECT id FROM {} ORDER BY id DESC", table_name)
}

/// Helper function to normalize MariaDB localhost to 127.0.0.1 for testing
pub fn normalize_mariadb_localhost_url(url: &str) -> Result<String> {
    let mut parsed_url = url::Url::parse(url)?;
    if parsed_url.host_str() == Some("localhost") {
        parsed_url.set_host(Some("127.0.0.1"))?;
    }
    Ok(parsed_url.to_string())
}