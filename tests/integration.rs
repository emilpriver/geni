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

use suitecase::{cases, run, Case, HookFns, RunConfig};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio::runtime::Runtime;

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

async fn test_migrate(database: Database, db_url: &str) -> Result<()> {
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
        "schema_migrations".to_string(),
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
        "schema_migrations".to_string(),
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
        "schema_migrations".to_string(),
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
        "schema_migrations".to_string(),
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
        "schema_migrations".to_string(),
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

struct PostgresSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl PostgresSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_postgres(s: &mut PostgresSuite) {
    let rt = Runtime::new().unwrap();
    let container = rt
        .block_on(
            GenericImage::new("postgres", "14.10")
                .with_exposed_port(5432.tcp())
                .with_wait_for(WaitFor::message_on_stdout(
                    "database system is ready to accept connections",
                ))
                .with_env_var("POSTGRES_DB", "development")
                .with_env_var("POSTGRES_USER", "postgres")
                .with_env_var("POSTGRES_PASSWORD", "mysecretpassword")
                .start(),
        )
        .expect("Failed to start postgres");
    let host_port = rt
        .block_on(container.get_host_port_ipv4(5432))
        .expect("Failed to get postgres port");
    let url = format!(
        "postgres://postgres:mysecretpassword@localhost:{}/app?sslmode=disable",
        host_port
    );
    s._container = Some(container);
    s.url = Some(url);
}

static POSTGRES_CASES: &[Case<PostgresSuite>] = cases![PostgresSuite, s =>
    test_migrate_postgres => {
        env::set_var("DATABASE_SCHEMA_FILE", "postgres_schema.sql");
        let url = s.url.as_ref().unwrap().clone();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(test_migrate(Database::Postgres, &url)).unwrap();
    },
];

fn run_postgres_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = PostgresSuite::new();
    let hook_fns = HookFns {
        setup_suite: Some(setup_postgres),
        before_each: None,
        after_each: None,
        teardown_suite: None,
    };
    run(&mut suite, POSTGRES_CASES, RunConfig::filter(name), &hook_fns);
    if let Some(container) = suite._container.take() {
        rt.block_on(async { drop(container); });
    }
}

#[test]
fn test_migrate_postgres() {
    run_postgres_test("test_migrate_postgres");
}

struct MySQLSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl MySQLSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_mysql(s: &mut MySQLSuite) {
    let rt = Runtime::new().unwrap();
    let container = rt
        .block_on(
            GenericImage::new("mysql", "latest")
                .with_exposed_port(3306.tcp())
                .with_wait_for(WaitFor::message_on_stdout(
                    "/usr/sbin/mysqld: ready for connections",
                ))
                .with_env_var("MYSQL_ROOT_PASSWORD", "password")
                .with_env_var("MYSQL_DATABASE", "development")
                .start(),
        )
        .expect("Failed to start mysql");
    let host_port = rt
        .block_on(container.get_host_port_ipv4(3306))
        .expect("Failed to get mysql port");
    let url = format!("mysql://root:password@localhost:{}/app", host_port);
    s._container = Some(container);
    s.url = Some(url);
}

static MYSQL_CASES: &[Case<MySQLSuite>] = cases![MySQLSuite, s =>
    test_migrate_mysql => {
        env::set_var("DATABASE_SCHEMA_FILE", "mysql_schema.sql");
        let url = s.url.as_ref().unwrap().clone();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(test_migrate(Database::MySQL, &url)).unwrap();
    },
];

fn run_mysql_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = MySQLSuite::new();
    let hook_fns = HookFns {
        setup_suite: Some(setup_mysql),
        before_each: None,
        after_each: None,
        teardown_suite: None,
    };
    run(&mut suite, MYSQL_CASES, RunConfig::filter(name), &hook_fns);
    if let Some(container) = suite._container.take() {
        rt.block_on(async { drop(container); });
    }
}

#[test]
fn test_migrate_mysql() {
    run_mysql_test("test_migrate_mysql");
}

struct MariaDBSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl MariaDBSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_mariadb(s: &mut MariaDBSuite) {
    let rt = Runtime::new().unwrap();
    let container = rt
        .block_on(
            GenericImage::new("mariadb", "11.1.3")
                .with_exposed_port(3306.tcp())
                .with_wait_for(WaitFor::message_on_stdout("port: 3306"))
                .with_env_var("MARIADB_ROOT_PASSWORD", "password")
                .with_env_var("MARIADB_DATABASE", "development")
                .start(),
        )
        .expect("Failed to start mariadb");
    let host_port = rt
        .block_on(container.get_host_port_ipv4(3306))
        .expect("Failed to get mariadb port");
    let url = format!("mariadb://root:password@localhost:{}/app", host_port);
    s._container = Some(container);
    s.url = Some(url);
}

static MARIADB_CASES: &[Case<MariaDBSuite>] = cases![MariaDBSuite, s =>
    test_migrate_maria => {
        env::set_var("DATABASE_SCHEMA_FILE", "maria_schema.sql");
        let url = s.url.as_ref().unwrap().clone();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(test_migrate(Database::MariaDB, &url)).unwrap();
    },
];

fn run_mariadb_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = MariaDBSuite::new();
    let hook_fns = HookFns {
        setup_suite: Some(setup_mariadb),
        before_each: None,
        after_each: None,
        teardown_suite: None,
    };
    run(&mut suite, MARIADB_CASES, RunConfig::filter(name), &hook_fns);
    if let Some(container) = suite._container.take() {
        rt.block_on(async { drop(container); });
    }
}

#[test]
fn test_migrate_maria() {
    run_mariadb_test("test_migrate_maria");
}

struct LibSQLSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl LibSQLSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_libsql(s: &mut LibSQLSuite) {
    let rt = Runtime::new().unwrap();
    let container = rt
        .block_on(
            GenericImage::new("ghcr.io/tursodatabase/libsql-server", "latest")
                .with_exposed_port(8080.tcp())
                .with_wait_for(WaitFor::message_on_stdout("listening on"))
                .start(),
        )
        .expect("Failed to start libsql");
    let host_port = rt
        .block_on(container.get_host_port_ipv4(8080))
        .expect("Failed to get libsql port");
    let url = format!("http://localhost:{}", host_port);
    s._container = Some(container);
    s.url = Some(url);
}

static LIBSQL_CASES: &[Case<LibSQLSuite>] = cases![LibSQLSuite, s =>
    test_migrate_libsql => {
        env::set_var("DATABASE_SCHEMA_FILE", "libsql_schema.sql");
        let url = s.url.as_ref().unwrap().clone();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(test_migrate(Database::LibSQL, &url)).unwrap();
    },
];

fn run_libsql_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = LibSQLSuite::new();
    let hook_fns = HookFns {
        setup_suite: Some(setup_libsql),
        before_each: None,
        after_each: None,
        teardown_suite: None,
    };
    run(&mut suite, LIBSQL_CASES, RunConfig::filter(name), &hook_fns);
    if let Some(container) = suite._container.take() {
        rt.block_on(async { drop(container); });
    }
}

#[test]
fn test_migrate_libsql() {
    run_libsql_test("test_migrate_libsql");
}

#[test]
fn test_migrate_sqlite() {
    env::set_var("DATABASE_SCHEMA_FILE", "sqlite_schema.sql");
    let tmp_dir = TempDir::new().unwrap();
    let migration_folder_string = tmp_dir.path().to_str().unwrap().to_string();
    let filename = format!("{}/test.sqlite", migration_folder_string);
    let path = std::path::Path::new(&filename);
    File::create(path).unwrap();
    let url = format!("sqlite://{}", path.to_str().unwrap());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(test_migrate(Database::SQLite, &url)).unwrap();
}

#[test]
fn test_migrate_failure() {
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

    let rt = tokio::runtime::Runtime::new().unwrap();
    let u = rt.block_on(up(
        url.clone(),
        None,
        "schema_migrations".to_string(),
        migration_folder_string.clone(),
        database_schema_file.clone(),
        Some(database_wait_timeout),
        true,
    ));
    assert!(u.is_err());
}
