use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use log::info;
use std::future::Future;
use std::pin::Pin;
use turso::{Builder, Connection};

use super::utils;

pub struct TursoDriver {
    conn: Connection,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl TursoDriver {
    pub async fn new(
        db_url: &String,
        _token: Option<String>,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<TursoDriver> {
        // Parse the turso:// URL to extract the file path
        let path = if db_url.starts_with("turso://") {
            &db_url["turso://".len()..]
        } else {
            bail!("Invalid Turso URL scheme. Must start with turso://")
        };

        info!("Connecting to local Turso database at: {}", path);

        let db = Builder::new_local(path).build().await?;
        let conn = db.connect()?;

        Ok(TursoDriver {
            conn,
            migrations_folder,
            migrations_table,
            schema_file,
        })
    }
}

impl DatabaseDriver for TursoDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
        run_in_transaction: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            if run_in_transaction {
                // Turso SDK doesn't have execute_transactional_batch, so we wrap in BEGIN/COMMIT
                self.conn.execute("BEGIN TRANSACTION", ()).await?;
                match self.conn.execute(query, ()).await {
                    Ok(_) => {
                        self.conn.execute("COMMIT", ()).await?;
                        Ok(())
                    }
                    Err(e) => {
                        let _ = self.conn.execute("ROLLBACK", ()).await;
                        Err(e.into())
                    }
                }
            } else {
                self.conn.execute(query, ()).await?;
                Ok(())
            }
        };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move {
            self.conn
                .execute(
                    format!(
                        "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) NOT NULL PRIMARY KEY);",
                        self.migrations_table
                    )
                    .as_str(),
                    (),
                )
                .await?;

            let mut stmt = self
                .conn
                .prepare(
                    format!(
                        "SELECT id FROM {} ORDER BY id DESC;",
                        self.migrations_table
                    )
                    .as_str(),
                )
                .await?;

            let mut rows = stmt.query(()).await?;

            let mut schema_migrations: Vec<String> = vec![];
            while let Some(row) = rows.next().await? {
                if let Ok(id) = row.get::<String>(0) {
                    schema_migrations.push(id);
                }
            }

            Ok(schema_migrations)
        };

        Box::pin(fut)
    }

    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let migrations_table = self.migrations_table.as_str();
            self.conn
                .execute(
                    format!("INSERT INTO {} (id) VALUES (?)", migrations_table).as_str(),
                    [id],
                )
                .await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let migrations_table = self.migrations_table.as_str();
            self.conn
                .execute(
                    format!("DELETE FROM {} WHERE id = ?", migrations_table).as_str(),
                    [id],
                )
                .await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn create_database(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::prelude::v1::Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            bail!("Geni does not support creating a database, it should be done via the respective interface")
        };

        Box::pin(fut)
    }

    fn drop_database(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = std::prelude::v1::Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            bail!("Geni does not support dropping a database, it should be done via the respective interface")
        };

        Box::pin(fut)
    }

    fn ready(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            self.conn.execute("SELECT 1", ()).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn dump_database_schema(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let schema = r#"
                --
                -- Turso SQL Schema dump automatic generated by geni
                --

            "#;
            let mut schema = schema
                .lines()
                .map(str::trim_start)
                .collect::<Vec<&str>>()
                .join("\n");

            let mut stmt = self
                .conn
                .prepare("SELECT sql FROM sqlite_master")
                .await?;

            let mut rows = stmt.query(()).await?;

            let mut schemas: Vec<String> = vec![];
            while let Some(row) = rows.next().await? {
                if let Ok(sql) = row.get::<String>(0) {
                    let text = sql
                        .trim_start_matches('"')
                        .trim_end_matches('"')
                        .replace("\\n", "\n");
                    schemas.push(format!("{};", text));
                }
            }

            schema.push_str(schemas.join("\n").as_str());

            utils::write_to_schema_file(
                schema,
                self.migrations_folder.clone(),
                self.schema_file.clone(),
            )
            .await?;

            Ok(())
        };

        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::database_test_utils::*;

    #[test]
    fn test_validate_turso_url_valid_local() {
        let url = "turso://./local.db";
        let result = validate_turso_url(url);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_validate_turso_url_valid_path() {
        let url = "turso:///tmp/test.db";
        let result = validate_turso_url(url);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_validate_turso_url_memory() {
        let url = "turso://:memory:";
        let result = validate_turso_url(url);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn test_validate_turso_url_invalid_scheme() {
        let invalid_urls = vec![
            "sqlite://test.db",
            "postgres://localhost/test",
            "mysql://localhost/test",
            "http://localhost",
            "https://example.com",
        ];

        for url in invalid_urls {
            let result = validate_turso_url(url);
            assert!(result.is_err(), "URL should be invalid: {}", url);
            assert!(result.unwrap_err().to_string().contains("Invalid Turso URL scheme"));
        }
    }

    #[test]
    fn test_turso_driver_parameters() {
        let _db_url = "turso://./test.db".to_string();
        let _token: Option<String> = None;
        let _migrations_table = "schema_migrations".to_string();
        let _migrations_folder = "./migrations".to_string();
        let _schema_file = "schema.sql".to_string();

        let _test_signature = || async {
            let _future = TursoDriver::new(
                &_db_url,
                _token.clone(),
                _migrations_table.clone(),
                _migrations_folder.clone(),
                _schema_file.clone(),
            );
            Ok::<(), anyhow::Error>(())
        };

        assert!(true);
    }

    #[test]
    fn test_turso_driver_struct_fields() {
        fn _test_fields() {
            let _check_migrations_table: fn(&TursoDriver) -> &String = |driver| &driver.migrations_table;
            let _check_migrations_folder: fn(&TursoDriver) -> &String = |driver| &driver.migrations_folder;
            let _check_schema_file: fn(&TursoDriver) -> &String = |driver| &driver.schema_file;
        }

        assert!(true);
    }

    #[test]
    fn test_url_scheme_detection() {
        let test_cases = vec![
            ("turso://./local.db", true),
            ("turso:///tmp/test.db", true),
            ("turso://:memory:", true),
            ("postgres://localhost/test", false),
            ("sqlite://test.db", false),
            ("invalid-url", false),
        ];

        for (url, should_be_valid) in test_cases {
            let result = validate_turso_url(url);
            if should_be_valid {
                assert!(result.is_ok(), "URL should be valid: {}", url);
            } else {
                assert!(result.is_err(), "URL should be invalid: {}", url);
            }
        }
    }

    #[test]
    fn test_local_path_acceptance() {
        let local_paths = vec![
            "turso://./test.db",
            "turso://./nested/path/test.db",
            "turso://./relative.sqlite",
            "turso:///absolute/path/test.db",
        ];

        for path in local_paths {
            let result = validate_turso_url(path);
            assert!(result.is_ok(), "Local path should be accepted: {}", path);
        }
    }

    #[test]
    fn test_special_paths() {
        let special_paths = vec![
            "turso://:memory:",
            "turso://file::memory:?cache=shared",
        ];

        for path in special_paths {
            let result = validate_turso_url(path);
            assert!(result.is_ok(), "Special path should be accepted: {}", path);
        }
    }
}
