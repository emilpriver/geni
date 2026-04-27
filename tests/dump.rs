use anyhow::Result;
use std::fs;
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

use geni::database_drivers;
use geni::dump::dump;

use suitecase::{cases, run, Case, HookFns, RunConfig};
use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};
use tokio::runtime::Runtime;

async fn setup_test_schema(database_url: &str) -> Result<()> {
    let mut create_client = database_drivers::new(
        database_url.to_string(),
        None,
        "schema_migrations".to_string(),
        "./migrations".to_string(),
        "schema.sql".to_string(),
        Some(30),
        false,
    )
    .await?;

    create_client.create_database().await?;

    drop(create_client);

    let mut client = database_drivers::new(
        database_url.to_string(),
        None,
        "schema_migrations".to_string(),
        "./migrations".to_string(),
        "schema.sql".to_string(),
        Some(30),
        true,
    )
    .await?;

    let test_queries = vec![
        "CREATE TABLE test_users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT UNIQUE);",
        "CREATE TABLE test_posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, content TEXT, user_id INTEGER);",
        "CREATE INDEX idx_posts_user_id ON test_posts(user_id);",
    ];

    for query in test_queries {
        let _ = client.execute(query, false).await;
    }

    Ok(())
}

async fn run_dump_test(database_url: &str, schema_file: &str, migrations_folder: &str) -> Result<()> {
    let result = dump(
        database_url.to_string(),
        None,
        "schema_migrations".to_string(),
        migrations_folder.to_string(),
        schema_file.to_string(),
        Some(30),
    )
    .await;

    assert!(result.is_ok(), "Dump should succeed");

    let schema_path = Path::new(migrations_folder).join(schema_file);
    assert!(schema_path.exists(), "Schema file should be created");

    let schema_content = fs::read_to_string(&schema_path)?;
    assert!(
        !schema_content.trim().is_empty(),
        "Schema file should not be empty"
    );

    Ok(())
}

struct PostgresDumpSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl PostgresDumpSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_postgres(s: &mut PostgresDumpSuite) {
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

static POSTGRES_CASES: &[Case<PostgresDumpSuite>] = cases![PostgresDumpSuite, s =>
    test_dump_postgres => {
        let url = s.url.as_ref().unwrap().clone();
        let tmp_dir = TempDir::new().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "postgres_dump_test_schema.sql";

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(setup_test_schema(&url)).unwrap();
        rt.block_on(run_dump_test(&url, schema_file, &migrations_folder)).unwrap();
    },
];

fn run_postgres_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = PostgresDumpSuite::new();
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
fn test_dump_postgres() {
    run_postgres_test("test_dump_postgres");
}

struct MySQLDumpSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl MySQLDumpSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_mysql(s: &mut MySQLDumpSuite) {
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

static MYSQL_CASES: &[Case<MySQLDumpSuite>] = cases![MySQLDumpSuite, s =>
    test_dump_mysql => {
        let url = s.url.as_ref().unwrap().clone();
        let tmp_dir = TempDir::new().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "mysql_dump_test_schema.sql";

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(setup_test_schema(&url)).unwrap();
        rt.block_on(run_dump_test(&url, schema_file, &migrations_folder)).unwrap();
    },
];

fn run_mysql_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = MySQLDumpSuite::new();
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
fn test_dump_mysql() {
    run_mysql_test("test_dump_mysql");
}

struct MariaDBDumpSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl MariaDBDumpSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_mariadb(s: &mut MariaDBDumpSuite) {
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

static MARIADB_CASES: &[Case<MariaDBDumpSuite>] = cases![MariaDBDumpSuite, s =>
    test_dump_mariadb => {
        let url = s.url.as_ref().unwrap().clone();
        let tmp_dir = TempDir::new().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "mariadb_dump_test_schema.sql";

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(setup_test_schema(&url)).unwrap();
        rt.block_on(run_dump_test(&url, schema_file, &migrations_folder)).unwrap();
    },
];

fn run_mariadb_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = MariaDBDumpSuite::new();
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
fn test_dump_mariadb() {
    run_mariadb_test("test_dump_mariadb");
}

struct LibSQLDumpSuite {
    _container: Option<ContainerAsync<GenericImage>>,
    url: Option<String>,
}

impl LibSQLDumpSuite {
    fn new() -> Self {
        Self {
            _container: None,
            url: None,
        }
    }
}

fn setup_libsql(s: &mut LibSQLDumpSuite) {
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

static LIBSQL_CASES: &[Case<LibSQLDumpSuite>] = cases![LibSQLDumpSuite, s =>
    test_dump_libsql => {
        let url = s.url.as_ref().unwrap().clone();
        let tmp_dir = TempDir::new().unwrap();
        let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
        let schema_file = "libsql_dump_test_schema.sql";

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(setup_test_schema(&url)).unwrap();
        rt.block_on(run_dump_test(&url, schema_file, &migrations_folder)).unwrap();
    },
];

fn run_libsql_test(name: &str) {
    let rt = Runtime::new().unwrap();
    let mut suite = LibSQLDumpSuite::new();
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
fn test_dump_libsql() {
    run_libsql_test("test_dump_libsql");
}

#[test]
fn test_dump_sqlite() {
    let tmp_dir = TempDir::new().unwrap();
    let db_file = tmp_dir.path().join("test_dump.sqlite");
    File::create(&db_file).unwrap();

    let database_url = format!("sqlite://{}", db_file.to_str().unwrap());
    let schema_file = "sqlite_dump_test_schema.sql";
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(setup_test_schema(&database_url)).unwrap();
    rt.block_on(run_dump_test(&database_url, schema_file, &migrations_folder))
        .unwrap();
}

#[test]
fn test_dump_with_invalid_database_url() {
    let tmp_dir = TempDir::new().unwrap();
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(dump(
        "invalid://database/url".to_string(),
        None,
        "schema_migrations".to_string(),
        migrations_folder,
        "schema.sql".to_string(),
        Some(30),
    ));

    assert!(result.is_err(), "Dump should fail with invalid database URL");
}
