use anyhow::{bail, Result};
use std::env;

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("DATABASE_MIGRATIONS_FOLDER") {
        return v;
    }

    "./migrations".to_string()
}

pub fn database_url() -> Result<String> {
    if let Ok(v) = env::var("DATABASE_URL") {
        return Ok(v);
    }

    bail!("missing DATABASE_URL env variable")
}

pub fn database_token() -> Option<String> {
    if let Ok(v) = env::var("DATABASE_TOKEN") {
        return Some(v);
    }

    None
}

pub fn wait_timeout() -> usize {
    if let Ok(v) = env::var("DATABASE_WAIT_TIMEOUT") {
        if let Ok(v) = v.parse::<usize>() {
            return v;
        }
    }

    0
}

pub enum Database {
    LibSQL,
    Postgres,
    MariaDB,
    MySQL,
    SQLite,
}

impl Database {
    pub fn new(s: &str) -> Result<Database> {
        match s {
            "https" | "http" => Ok(Database::LibSQL),
            "psql" | "postgres" => Ok(Database::Postgres),
            "mariadb" => Ok(Database::MariaDB),
            "mysql" => Ok(Database::MySQL),
            "sqlite" | "sqlite3" => Ok(Database::SQLite),
            _ => bail!("Unknown database driver"),
        }
    }

    pub fn as_str(&self) -> Result<&str> {
        match self {
            Database::LibSQL => Ok("libsql"),
            Database::Postgres => Ok("postgres"),
            Database::MariaDB => Ok("mariadb"),
            Database::MySQL => Ok("mysql"),
            Database::SQLite => Ok("sqlite"),
            _ => bail!("Unknown database driver"),
        }
    }
}
