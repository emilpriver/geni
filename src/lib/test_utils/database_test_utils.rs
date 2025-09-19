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