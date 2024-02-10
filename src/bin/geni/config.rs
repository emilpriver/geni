use anyhow::{bail, Result};
use std::env;

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("DATABASE_MIGRATIONS_FOLDER") {
        if !v.is_empty() {
            return v;
        }
    }

    "./migrations".to_string()
}

pub fn database_url() -> Result<String> {
    if let Ok(v) = env::var("DATABASE_URL") {
        if !v.is_empty() {
            return Ok(v);
        }
    }

    bail!("missing DATABASE_URL env variable")
}

pub fn database_token() -> Option<String> {
    if let Ok(v) = env::var("DATABASE_TOKEN") {
        if !v.is_empty() {
            return Some(v);
        }
    }

    None
}

pub fn wait_timeout() -> usize {
    if let Ok(v) = env::var("DATABASE_WAIT_TIMEOUT") {
        if !v.is_empty() {
            if let Ok(v) = v.parse::<usize>() {
                return v;
            }
        }
    }

    30
}

pub fn dump_schema_file() -> bool {
    if let Ok(v) = env::var("DATABASE_NO_DUMP_SCHEMA") {
        if v == "true" {
            return false;
        }
    }

    true
}

pub fn schema_file() -> String {
    if let Ok(v) = env::var("DATABASE_SCHEMA_FILE") {
        if !v.is_empty() {
            return v;
        }
    }

    "schema.sql".to_string()
}

pub fn migrations_table() -> String {
    if let Ok(v) = env::var("DATABASE_MIGRATIONS_TABLE") {
        if !v.is_empty() {
            return v;
        }
    }

    "schema_migrations".to_string()
}
