#[cfg(test)]
mod tests {
    use crate::config::Database;
    use crate::database_drivers;
    use log::info;
    use serial_test::serial;

    use crate::migrate::{down, up};
    use anyhow::Ok;
    use anyhow::Result;
    use chrono::Utc;
    use std::fs;
    use std::io::Write;
    use std::{env, fs::File, vec};
    use tokio::test;

    fn generate_test_migrations(migration_path: String) -> Result<()> {
        let file_endings = vec!["up", "down"];
        let test_queries = [
            (
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
                "DROP TABLE users;",
            ),
            (
                "CREATE TABLE users2 (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
                "DROP TABLE users2;",
            ),
            (
                "CREATE TABLE users3 (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
                "DROP TABLE users3;",
            ),
            (
                "CREATE TABLE users4 (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
                "DROP TABLE users4;",
            ),
            (
                "CREATE TABLE users5 (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
                "DROP TABLE users5;",
            ),
            (
                "CREATE TABLE users6 (id INTEGER PRIMARY KEY, name TEXT NOT NULL);",
                "DROP TABLE users6;",
            ),
        ];

        for (index, t) in test_queries.iter().enumerate() {
            for f in &file_endings {
                let timestamp = Utc::now().timestamp() + index as i64;

                let filename = format!("{migration_path}/{timestamp}_{index}_test.{f}.sql");
                let filename_str = filename.as_str();
                let path = std::path::Path::new(filename_str);

                // Generate the folder if it don't exist
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }

                let mut file = File::create(path)?;

                match *f {
                    "up" => {
                        file.write_all(t.0.as_bytes())?;
                    }
                    "down" => {
                        file.write_all(t.1.as_bytes())?;
                    }
                    _ => {}
                }

                info!("Generated {}", filename_str)
            }
        }

        Ok(())
    }

    async fn test_migrate(database: Database, url: &str) -> Result<()> {
        let tmp_dir =
            tempdir::TempDir::new(format!("test_migrate_{}", database.as_str().unwrap()).as_str())
                .unwrap();
        let migration_folder = tmp_dir.path();
        let migration_folder_string = migration_folder.to_str().unwrap();

        env::set_var("DATABASE_WAIT_TIMEOUT", "30");
        env::set_var("DATABASE_MIGRATIONS_FOLDER", migration_folder_string);

        generate_test_migrations(migration_folder_string.to_string()).unwrap();

        env::set_var("DATABASE_TOKEN", "not needed");
        env::set_var("DATABASE_URL", url);

        let mut create_client = database_drivers::new(url, false).await.unwrap();
        match database {
            // we don't need to match sqlite as sqlite driver creates the file if it doesn't exist
            Database::Postgres | Database::MySQL | Database::MariaDB => {
                create_client.create_database().await.unwrap();
            }
            _ => {}
        };

        let mut client = database_drivers::new(url, true).await.unwrap();

        let u = up().await;
        assert!(u.is_ok());

        assert_eq!(
            client
                .get_or_create_schema_migrations()
                .await
                .unwrap()
                .len(),
            6,
        );

        let d = down(&1).await;
        assert!(d.is_ok());

        assert_eq!(
            client
                .get_or_create_schema_migrations()
                .await
                .unwrap()
                .len(),
            5
        );

        let d = down(&3).await;
        assert!(d.is_ok());

        assert_eq!(
            client
                .get_or_create_schema_migrations()
                .await
                .unwrap()
                .len(),
            2
        );

        Ok(())
    }

    #[test]
    #[serial]
    async fn test_migrate_libsql() -> Result<()> {
        let url = "http://localhost:6000";
        test_migrate(Database::LibSQL, url).await
    }

    #[test]
    #[serial]
    async fn test_migrate_postgres() -> Result<()> {
        let url = "psql://postgres:mysecretpassword@localhost:6437/app?sslmode=disable";
        test_migrate(Database::Postgres, url).await
    }

    #[test]
    #[serial]
    async fn test_migrate_mysql() -> Result<()> {
        let url = "mysql://root:password@localhost:3306/app";
        test_migrate(Database::MySQL, url).await
    }

    #[test]
    #[serial]
    async fn test_migrate_maria() -> Result<()> {
        let url = "mariadb://root:password@localhost:3307/app";
        test_migrate(Database::MariaDB, url).await
    }

    #[test]
    #[serial]
    async fn test_migrate_sqlite() -> Result<()> {
        let tmp_dir = tempdir::TempDir::new("temp_migrate_sqlite_db").unwrap();
        let migration_folder = tmp_dir.path();
        let migration_folder_string = migration_folder.to_str().unwrap();
        let filename = format!("{migration_folder_string}/test.sqlite");
        let filename_str = filename.as_str();
        let path = std::path::Path::new(filename_str);

        File::create(path)?;

        let url = format!("sqlite://{}", path.to_str().unwrap());

        test_migrate(Database::SQLite, &url).await
    }
}
