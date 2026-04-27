use anyhow::Result;
use chrono::Utc;
use log::info;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

use geni::config::Database;
use geni::database_drivers;
use geni::migrate::{down, up};

use testcontainers::core::wait::LogWaitStrategy;
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};

fn generate_test_migrations(migration_path: &str) -> Result<()> {
    let file_endings = vec!["up", "down"];
    let test_queries = [
        (
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL); CREATE TABLE computers (id INTEGER PRIMARY KEY, name TEXT NOT NULL); ;",
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
            let path = std::path::Path::new(filename.as_str());

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            let mut file = File::create(path)?;
            match *f {
                "up" => file.write_all(t.0.as_bytes())?,
                "down" => file.write_all(t.1.as_bytes())?,
                _ => {}
            }
            info!("Generated {}", filename)
        }
    }

    Ok(())
}

async fn test_migrate(database: Database, db_url: &str, migrations_table: &str) -> Result<()> {
    let url = db_url.to_string();
    let tmp_dir = TempDir::new()?;
    let migration_folder_string = tmp_dir.path().to_str().unwrap().to_string();
    let database_wait_timeout = 30;
    let database_schema_file =
        env::var("DATABASE_SCHEMA_FILE").unwrap_or("schema.sql".to_string());

    generate_test_migrations(&migration_folder_string)?;

    let mut create_client = database_drivers::new(
        url.clone(),
        None,
        migrations_table.to_string(),
        migration_folder_string.clone(),
        database_schema_file.clone(),
        Some(database_wait_timeout),
        false,
    )
    .await
    .unwrap();

    match database {
        Database::Postgres | Database::MySQL | Database::MariaDB => {
            create_client.create_database().await.unwrap();
        }
        _ => {}
    };

    let mut client = database_drivers::new(
        url.clone(),
        None,
        migrations_table.to_string(),
        migration_folder_string.clone(),
        database_schema_file.clone(),
        Some(database_wait_timeout),
        true,
    )
    .await
    .unwrap();

    let u = up(
        url.clone(),
        None,
        migrations_table.to_string(),
        migration_folder_string.clone(),
        database_schema_file.clone(),
        Some(database_wait_timeout),
        true,
    )
    .await;
    assert!(u.is_ok());
    assert_eq!(
        client
            .get_or_create_schema_migrations()
            .await
            .unwrap()
            .len(),
        6,
    );

    let d = down(
        url.clone(),
        None,
        migrations_table.to_string(),
        migration_folder_string.clone(),
        database_schema_file.clone(),
        Some(database_wait_timeout),
        false,
        &1,
    )
    .await;
    assert!(d.is_ok());
    assert_eq!(
        client
            .get_or_create_schema_migrations()
            .await
            .unwrap()
            .len(),
        5
    );

    let d = down(
        url,
        None,
        migrations_table.to_string(),
        migration_folder_string.clone(),
        database_schema_file.clone(),
        Some(database_wait_timeout),
        false,
        &3,
    )
    .await;
    assert!(d.is_ok());
    assert_eq!(
        client
            .get_or_create_schema_migrations()
            .await
            .unwrap()
            .len(),
        2
    );

    let schema_dump_file = format!("{}/{}", migration_folder_string, database_schema_file);
    assert!(Path::new(&schema_dump_file).exists());

    Ok(())
}

#[tokio::test]
async fn test_migrate_postgres() -> Result<()> {
    let container = GenericImage::new("postgres", "18.0")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_DB", "development")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "mysecretpassword")
        .start()
        .await
        .expect("Failed to start postgres");
    let host_port = container.get_host_port_ipv4(5432).await?;
    let url = format!(
        "postgres://postgres:mysecretpassword@localhost:{}/app?sslmode=disable",
        host_port
    );

    env::set_var("DATABASE_SCHEMA_FILE", "postgres_schema.sql");
    test_migrate(Database::Postgres, &url, "schema_migrations").await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_migrate_postgres_schema_qualified() -> Result<()> {
    let container = GenericImage::new("postgres", "18.0")
        .with_exposed_port(5432.tcp())
        .with_wait_for(WaitFor::message_on_stdout(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_DB", "development")
        .with_env_var("POSTGRES_USER", "postgres")
        .with_env_var("POSTGRES_PASSWORD", "mysecretpassword")
        .start()
        .await
        .expect("Failed to start postgres");
    let host_port = container.get_host_port_ipv4(5432).await?;
    let url = format!(
        "postgres://postgres:mysecretpassword@localhost:{}/app?sslmode=disable",
        host_port
    );

    env::set_var("DATABASE_SCHEMA_FILE", "postgres_schema.sql");
    test_migrate(Database::Postgres, &url, "migrations.migrations").await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_migrate_mysql() -> Result<()> {
    let container = GenericImage::new("mysql", "latest")
        .with_exposed_port(3306.tcp())
        .with_wait_for(WaitFor::Log(
            LogWaitStrategy::stdout_or_stderr("ready for connections").with_times(2),
        ))
        .with_env_var("MYSQL_ROOT_PASSWORD", "password")
        .with_env_var("MYSQL_DATABASE", "development")
        .start()
        .await
        .expect("Failed to start mysql");
    let host_port = container.get_host_port_ipv4(3306).await?;
    let url = format!("mysql://root:password@localhost:{}/app", host_port);

    env::set_var("DATABASE_SCHEMA_FILE", "mysql_schema.sql");
    test_migrate(Database::MySQL, &url, "schema_migrations").await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_migrate_maria() -> Result<()> {
    let container = GenericImage::new("mariadb", "11.1.3")
        .with_exposed_port(3306.tcp())
        .with_wait_for(WaitFor::Log(
            LogWaitStrategy::stdout_or_stderr("ready for connections").with_times(2),
        ))
        .with_env_var("MARIADB_ROOT_PASSWORD", "password")
        .with_env_var("MARIADB_DATABASE", "development")
        .start()
        .await
        .expect("Failed to start mariadb");
    let host_port = container.get_host_port_ipv4(3306).await?;
    let url = format!("mariadb://root:password@localhost:{}/app", host_port);

    env::set_var("DATABASE_SCHEMA_FILE", "maria_schema.sql");
    test_migrate(Database::MariaDB, &url, "schema_migrations").await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_migrate_libsql() -> Result<()> {
    let container = GenericImage::new("ghcr.io/tursodatabase/libsql-server", "latest")
        .with_exposed_port(8080.tcp())
        .with_wait_for(WaitFor::message_on_either_std(
            "listening for incoming user HTTP connection",
        ))
        .start()
        .await
        .expect("Failed to start libsql");
    let host_port = container.get_host_port_ipv4(8080).await?;
    let url = format!("http://localhost:{}", host_port);

    env::set_var("DATABASE_SCHEMA_FILE", "libsql_schema.sql");
    test_migrate(Database::LibSQL, &url, "schema_migrations").await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_migrate_sqlite() {
    env::set_var("DATABASE_SCHEMA_FILE", "sqlite_schema.sql");
    let tmp_dir = TempDir::new().unwrap();
    let migration_folder_string = tmp_dir.path().to_str().unwrap().to_string();
    let filename = format!("{}/test.sqlite", migration_folder_string);
    let path = std::path::Path::new(&filename);
    File::create(path).unwrap();
    let url = format!("sqlite://{}", path.to_str().unwrap());
    test_migrate(Database::SQLite, &url, "schema_migrations").await.unwrap();
}

#[tokio::test]
async fn test_migrate_failure() {
    env::set_var("DATABASE_SCHEMA_FILE", "sqlite_schema.sql");
    let tmp_dir = TempDir::new().unwrap();
    let migration_folder_string = tmp_dir.path().to_str().unwrap().to_string();
    let filename = format!("{}/test.sqlite", migration_folder_string);
    let path = std::path::Path::new(&filename);
    File::create(path).unwrap();
    let url = format!("sqlite://{}", path.to_str().unwrap());

    let file_endings = vec!["up", "down"];
    let test_queries = [(
        r#"
            CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

            CREATE OR REPLACE FUNCTION uuid_generate_v7()
            RETURN uuid
            AS $$
            SELECT encode(
                set_bit(
                set_bit(
                    overlay(uuid_send(gen_random_uuid())
                            placing substring(int8send(floor(extract(epoch from clock_timestamp()) * 1000)::bigint) from 3)
                            from 1 for 6
                    ),
                    52, 1
                ),
                53, 1
                ),
                'hex')::uuid;
            $$
            LANGUAGE SQL
            VOLATILE;

            CREATE TABLE tokens (
                id UUID NOT NULL DEFAULT uuid_generate_v7 ()
                token TEXT NOT NULL UNIQUE,
                app_id UUID NOT NULL,
                app_slug TEXT NOT NULL,
                expires_at TIMESTAMP,
                updated_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE permissions (
                id UUID NOT NULL DEFAULT uuid_generate_v7 ()
                token UUID NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
            );
        "#,
        "",
    )];

    for (index, t) in test_queries.iter().enumerate() {
        for f in &file_endings {
            let timestamp = Utc::now().timestamp() + index as i64;
            let filename = format!("{migration_folder_string}/{timestamp}_{index}_test.{f}.sql");
            let path = std::path::Path::new(filename.as_str());

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap()
            }

            let mut file = File::create(path).unwrap();
            match *f {
                "up" => file.write_all(t.0.as_bytes()).unwrap(),
                "down" => file.write_all(t.1.as_bytes()).unwrap(),
                _ => {}
            }
            info!("Generated {}", filename)
        }
    }

    let database_wait_timeout = 30;
    let database_schema_file = "sqlite_schema.sql".to_string();

    let u = up(
        url.clone(),
        None,
        "schema_migrations".to_string(),
        migration_folder_string.clone(),
        database_schema_file.clone(),
        Some(database_wait_timeout),
        true,
    )
    .await;
    assert!(u.is_err());
}
