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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_new_valid_schemes() {
        // Test LibSQL schemes
        assert!(matches!(Database::new("https").unwrap(), Database::LibSQL));
        assert!(matches!(Database::new("http").unwrap(), Database::LibSQL));
        assert!(matches!(Database::new("libsql").unwrap(), Database::LibSQL));

        // Test Postgres schemes
        assert!(matches!(Database::new("psql").unwrap(), Database::Postgres));
        assert!(matches!(Database::new("postgres").unwrap(), Database::Postgres));
        assert!(matches!(Database::new("postgresql").unwrap(), Database::Postgres));

        // Test MariaDB scheme
        assert!(matches!(Database::new("mariadb").unwrap(), Database::MariaDB));

        // Test MySQL scheme
        assert!(matches!(Database::new("mysql").unwrap(), Database::MySQL));

        // Test SQLite schemes
        assert!(matches!(Database::new("sqlite").unwrap(), Database::SQLite));
        assert!(matches!(Database::new("sqlite3").unwrap(), Database::SQLite));
    }

    #[test]
    fn test_database_new_invalid_scheme() {
        assert!(Database::new("invalid").is_err());
        assert!(Database::new("").is_err());
        assert!(Database::new("mongodb").is_err());
        assert!(Database::new("redis").is_err());

        // Test case sensitivity
        assert!(Database::new("POSTGRES").is_err());
        assert!(Database::new("MySQL").is_err());
    }

    #[test]
    fn test_database_as_str() {
        assert_eq!(Database::LibSQL.as_str().unwrap(), "libsql");
        assert_eq!(Database::Postgres.as_str().unwrap(), "postgres");
        assert_eq!(Database::MariaDB.as_str().unwrap(), "mariadb");
        assert_eq!(Database::MySQL.as_str().unwrap(), "mysql");
        assert_eq!(Database::SQLite.as_str().unwrap(), "sqlite");
    }

    #[test]
    fn test_database_roundtrip() {
        // Test that we can create a Database and convert it back to string
        let schemes = vec!["libsql", "postgres", "mariadb", "mysql", "sqlite"];

        for scheme in schemes {
            let db = Database::new(scheme).unwrap();
            assert_eq!(db.as_str().unwrap(), scheme);
        }
    }
}
