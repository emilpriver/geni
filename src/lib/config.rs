use anyhow::{bail, Result};

pub enum Database {
    LibSQL,
    Postgres,
    MariaDB,
    MySQL,
    SQLite,
}

#[allow(dead_code)]
impl Database {
    pub fn new(s: &str) -> Result<Database> {
        match s {
            "https" | "http" | "libsql" => Ok(Database::LibSQL),
            "psql" | "postgres" | "postgresql" => Ok(Database::Postgres),
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
        }
    }
}
