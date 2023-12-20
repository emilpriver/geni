use crate::config::{database_driver, database_url, migration_folder};
use crate::database_drivers;
use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;
use std::vec;

fn get_paths(folder: &PathBuf, ending: &str) -> Vec<(i64, PathBuf)> {
    let entries = fs::read_dir(folder).unwrap();

    let mut migration_files = vec![];
    let end = format!(".{}.sql", ending);

    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();

        if entry.file_name().to_str().unwrap().ends_with(&end) {
            migration_files.push((path.clone(), path));
        }
    }

    let mut sorted = migration_files
        .iter()
        .map(|(path, _)| {
            let filename = path.file_name().unwrap().to_str().unwrap();
            let timestamp = filename.split_once("_").unwrap().0;
            let timestamp = timestamp.parse::<i64>().unwrap();

            (timestamp, path.clone())
        })
        .collect::<Vec<(i64, PathBuf)>>();

    sorted.sort_by(|a, b| a.0.cmp(&b.0));

    sorted
}

fn read_file_content(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

pub async fn up() -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = get_paths(&path, "up");

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .up.sql at {}. Does the path exist?",
            folder
        );
    }

    let database_url = database_url()?;
    let driver = database_driver()?;
    let database = database_drivers::new(driver, &database_url).await?;

    let migrations: Vec<String> = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.into_boxed_str())
        .collect::<Vec<Box<str>>>()
        .into_iter()
        .map(|s| s.into())
        .collect();

    for f in files {
        let id = Box::new(f.0.to_string());

        if !migrations.contains(&id) {
            println!("Running migration {}", id);
            let query = read_file_content(&f.1);

            database.execute(&query).await?;

            database.insert_schema_migration(&id).await?;
        }
    }

    Ok(())
}

pub async fn down(rollback_amount: &i64) -> Result<()> {
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = get_paths(&path, "down");

    if files.is_empty() {
        bail!(
            "Didn't find any files ending with .down.sql at {}. Does the path exist?",
            folder
        );
    }

    let database_url = database_url()?;
    let driver = database_driver()?;
    let database = database_drivers::new(driver, &database_url).await?;

    let migrations = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.into_boxed_str().parse::<i64>().unwrap())
        .collect::<Vec<i64>>();

    let migrations_to_run = migrations.into_iter().take(*rollback_amount as usize);

    for migration in migrations_to_run {
        let rollback_file = files.iter().find(|(timestamp, _)| timestamp == &migration);

        match rollback_file {
            None => bail!("No rollback file found for {}", migration),
            Some(f) => {
                println!("Running rollback for {}", migration);
                let query = read_file_content(&f.1);

                database.execute(&query).await?;

                database
                    .remove_schema_migration(&migration.to_string().as_str())
                    .await?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::Database;

    use super::*;
    use chrono::Utc;
    use std::io::Write;
    use std::{env, fs::File, vec};
    use tokio::test;

    fn generate_test_migrations(migration_path: String) -> Result<()> {
        let file_endings = vec!["up", "down"];
        let test_queries = vec![
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

                println!("Generated {}", filename_str)
            }
        }

        Ok(())
    }

    #[test]
    async fn test_migrate() {
        let tmp_dir = tempdir::TempDir::new("test_migrate").unwrap();
        let migration_folder = tmp_dir.path();
        let migration_folder_string = migration_folder.to_str().unwrap();

        env::set_var("MIGRATION_DIR", migration_folder_string);

        generate_test_migrations(migration_folder_string.to_string()).unwrap();

        let db_urls = vec![(Database::LibSQL, "http://localhost:6000")];

        env::set_var("DATABASE_TOKEN", "not needed");

        for url in db_urls {
            env::set_var("DATABASE", url.0.as_str());
            env::set_var("DATABASE_URL", url.1);

            let client = database_drivers::new(url.0, url.1).await.unwrap();

            let u = up().await;
            assert_eq!(u.is_ok(), true);

            let current_migrations = client
                .get_or_create_schema_migrations()
                .await
                .unwrap()
                .len();

            assert_eq!(current_migrations, 6);

            let d = down(&1).await;
            assert_eq!(d.is_ok(), true);

            let current_migrations = client
                .get_or_create_schema_migrations()
                .await
                .unwrap()
                .len();
            assert_eq!(current_migrations, 5);

            let d = down(&3).await;
            assert_eq!(d.is_ok(), true);

            let current_migrations = client
                .get_or_create_schema_migrations()
                .await
                .unwrap()
                .len();
            assert_eq!(current_migrations, 2);
        }
    }
}
