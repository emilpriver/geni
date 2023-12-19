use anyhow::{bail, Result};
use std::env;

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("MIGRATION_DIR") {
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

pub fn database_token() -> Result<String> {
    if let Ok(v) = env::var("DATABASE_TOKEN") {
        return Ok(v);
    }

    bail!("missing DATABASE_TOKEN env variable")
}

pub enum Database {
    LibSQL,
    Postgres,
    MariaDB,
    MySQL,
    SQLite,
}

impl Database {
    pub fn as_str(&self) -> &str {
        match self {
            Database::LibSQL => "libsql",
            Database::Postgres => "postgres",
            Database::MariaDB => "mariadb",
            Database::MySQL => "mysql",
            Database::SQLite => "sqlite",
        }
    }
}

pub fn database_driver() -> Result<Database> {
    if let Ok(v) = env::var("DATABASE") {
        match v.as_str() {
            "libsql" => return Ok(Database::LibSQL),
            "postgres" => return Ok(Database::Postgres),
            "mariadb" => return Ok(Database::MariaDB),
            "mysql" => return Ok(Database::MySQL),
            "sqlite" => return Ok(Database::SQLite),
            "sqlite3" => return Ok(Database::SQLite),
            _ => bail!("Unknown database driver"),
        }
    }

    bail!("missing DATABASE env variable")
}
