use anyhow::Result;
use std::fs;
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

use geni::database_drivers;
use geni::dump::dump;

use testcontainers::core::{IntoContainerPort, WaitFor};
use testcontainers::runners::AsyncRunner;
use testcontainers::{GenericImage, ImageExt};

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

#[tokio::test]
async fn test_dump_postgres() -> Result<()> {
    let container = GenericImage::new("postgres", "14.10")
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

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_test_schema(&url).await?;
    run_dump_test(&url, "postgres_dump_test_schema.sql", &migrations_folder).await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_dump_mysql() -> Result<()> {
    let container = GenericImage::new("mysql", "latest")
        .with_exposed_port(3306.tcp())
        .with_wait_for(WaitFor::message_on_stdout(
            "/usr/sbin/mysqld: ready for connections",
        ))
        .with_env_var("MYSQL_ROOT_PASSWORD", "password")
        .with_env_var("MYSQL_DATABASE", "development")
        .start()
        .await
        .expect("Failed to start mysql");
    let host_port = container.get_host_port_ipv4(3306).await?;
    let url = format!("mysql://root:password@localhost:{}/app", host_port);

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_test_schema(&url).await?;
    run_dump_test(&url, "mysql_dump_test_schema.sql", &migrations_folder).await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_dump_mariadb() -> Result<()> {
    let container = GenericImage::new("mariadb", "11.1.3")
        .with_exposed_port(3306.tcp())
        .with_wait_for(WaitFor::message_on_stdout("port: 3306"))
        .with_env_var("MARIADB_ROOT_PASSWORD", "password")
        .with_env_var("MARIADB_DATABASE", "development")
        .start()
        .await
        .expect("Failed to start mariadb");
    let host_port = container.get_host_port_ipv4(3306).await?;
    let url = format!("mariadb://root:password@localhost:{}/app", host_port);

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_test_schema(&url).await?;
    run_dump_test(&url, "mariadb_dump_test_schema.sql", &migrations_folder).await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_dump_libsql() -> Result<()> {
    let container = GenericImage::new("ghcr.io/tursodatabase/libsql-server", "latest")
        .with_exposed_port(8080.tcp())
        .with_wait_for(WaitFor::message_on_stdout("listening on"))
        .start()
        .await
        .expect("Failed to start libsql");
    let host_port = container.get_host_port_ipv4(8080).await?;
    let url = format!("http://localhost:{}", host_port);

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_test_schema(&url).await?;
    run_dump_test(&url, "libsql_dump_test_schema.sql", &migrations_folder).await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_dump_sqlite() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let db_file = tmp_dir.path().join("test_dump.sqlite");
    File::create(&db_file)?;

    let database_url = format!("sqlite://{}", db_file.to_str().unwrap());
    let schema_file = "sqlite_dump_test_schema.sql";
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();

    setup_test_schema(&database_url).await?;
    run_dump_test(&database_url, schema_file, &migrations_folder).await?;

    Ok(())
}

#[tokio::test]
async fn test_dump_with_invalid_database_url() {
    let tmp_dir = TempDir::new().unwrap();
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();

    let result = dump(
        "invalid://database/url".to_string(),
        None,
        "schema_migrations".to_string(),
        migrations_folder,
        "schema.sql".to_string(),
        Some(30),
    )
    .await;

    assert!(result.is_err(), "Dump should fail with invalid database URL");
}
