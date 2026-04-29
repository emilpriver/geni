use anyhow::Result;
use std::fs;
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

use geni::database_drivers;
use geni::dump::dump;

use testcontainers::core::wait::LogWaitStrategy;
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

    if !database_url.starts_with("http://") && !database_url.starts_with("https://") {
        create_client.create_database().await?;
    }

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
        "CREATE TABLE test_posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, content TEXT, user_id INTEGER REFERENCES test_users(id) ON DELETE CASCADE);",
        "CREATE INDEX idx_posts_user_id ON test_posts(user_id);",
    ];

    for query in test_queries {
        let _ = client.execute(query, false).await;
    }

    Ok(())
}

async fn run_dump_test(
    database_url: &str,
    schema_file: &str,
    migrations_folder: &str,
) -> Result<()> {
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
        .with_wait_for(WaitFor::message_on_either_std(
            "listening for incoming user HTTP connection",
        ))
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

    assert!(
        result.is_err(),
        "Dump should fail with invalid database URL"
    );
}

async fn setup_composite_pk_schema(database_url: &str) -> Result<()> {
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

    if !database_url.starts_with("http://") && !database_url.starts_with("https://") {
        create_client.create_database().await?;
    }

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

    let is_mysql = database_url.starts_with("mysql://") || database_url.starts_with("mariadb://");
    let create_table = if is_mysql {
        "CREATE TABLE fancy_pants (country VARCHAR(255), segment VARCHAR(255), slug VARCHAR(255), PRIMARY KEY (country, segment, slug));"
    } else {
        "CREATE TABLE fancy_pants (country TEXT, segment TEXT, slug TEXT, PRIMARY KEY (country, segment, slug));"
    };

    client.execute(create_table, false).await?;

    Ok(())
}

async fn run_composite_pk_dump_test(
    database_url: &str,
    schema_file: &str,
    migrations_folder: &str,
) -> Result<()> {
    dump(
        database_url.to_string(),
        None,
        "schema_migrations".to_string(),
        migrations_folder.to_string(),
        schema_file.to_string(),
        Some(30),
    )
    .await?;

    let schema_path = Path::new(migrations_folder).join(schema_file);
    assert!(schema_path.exists(), "Schema file should be created");

    let schema_content = fs::read_to_string(&schema_path)?;

    let is_sqlite = database_url.starts_with("sqlite://");

    if is_sqlite {
        let create_lines: Vec<&str> = schema_content
            .lines()
            .filter(|l| {
                l.contains("CREATE TABLE") && l.contains("fancy_pants") && l.contains("PRIMARY KEY")
            })
            .collect();

        assert_eq!(
            create_lines.len(),
            1,
            "Should have exactly one CREATE TABLE line with PRIMARY KEY, got {} lines: {:?}",
            create_lines.len(),
            create_lines
        );

        let create_line = create_lines[0];
        let pk_part = create_line
            .split("PRIMARY KEY")
            .nth(1)
            .unwrap()
            .split('(')
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        let pk_columns: Vec<&str> = pk_part.split(',').map(|s| s.trim()).collect();

        assert_eq!(
            pk_columns.len(),
            3,
            "Composite primary key should have 3 columns, got {}: {:?}\nCREATE TABLE line: {}\nPK part: '{}'",
            pk_columns.len(),
            pk_columns,
            create_line,
            pk_part
        );
        assert!(
            pk_columns.iter().any(|c| c.to_lowercase() == "country"),
            "Should contain 'country', got: {:?}",
            pk_columns
        );
        assert!(
            pk_columns.iter().any(|c| c.to_lowercase() == "segment"),
            "Should contain 'segment', got: {:?}",
            pk_columns
        );
        assert!(
            pk_columns.iter().any(|c| c.to_lowercase() == "slug"),
            "Should contain 'slug', got: {:?}",
            pk_columns
        );
        assert!(
            pk_columns.iter().any(|c| c.to_lowercase() == "country"),
            "Should contain 'country', got: {:?}",
            pk_columns
        );
        assert!(
            pk_columns.iter().any(|c| c.to_lowercase() == "segment"),
            "Should contain 'segment', got: {:?}",
            pk_columns
        );
        assert!(
            pk_columns.iter().any(|c| c.to_lowercase() == "slug"),
            "Should contain 'slug', got: {:?}",
            pk_columns
        );
    } else {
        let constraint_lines: Vec<&str> = schema_content
            .lines()
            .filter(|l| l.contains("fancy_pants") && l.contains("PRIMARY KEY"))
            .collect();

        assert_eq!(
            constraint_lines.len(),
            1,
            "Should have exactly one PRIMARY KEY constraint line, got {} lines: {:?}\nFull schema:\n{}",
            constraint_lines.len(),
            constraint_lines,
            schema_content
        );

        let constraint = constraint_lines[0];
        let pk_part = constraint
            .split("PRIMARY KEY")
            .nth(1)
            .unwrap()
            .split('(')
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        let pk_columns: Vec<&str> = pk_part.split(',').map(|s| s.trim()).collect();

        assert_eq!(
            pk_columns.len(),
            3,
            "Composite primary key should have 3 columns, got {}: {:?}",
            pk_columns.len(),
            pk_columns
        );
        assert!(pk_columns.contains(&"country"));
        assert!(pk_columns.contains(&"segment"));
        assert!(pk_columns.contains(&"slug"));
    }

    Ok(())
}

#[tokio::test]
async fn test_dump_composite_primary_key_postgres() -> Result<()> {
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

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_composite_pk_schema(&url).await?;
    run_composite_pk_dump_test(&url, "postgres_composite_pk_schema.sql", &migrations_folder)
        .await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_dump_composite_primary_key_mysql() -> Result<()> {
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

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_composite_pk_schema(&url).await?;
    run_composite_pk_dump_test(&url, "mysql_composite_pk_schema.sql", &migrations_folder).await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_dump_composite_primary_key_mariadb() -> Result<()> {
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

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_composite_pk_schema(&url).await?;
    run_composite_pk_dump_test(&url, "mariadb_composite_pk_schema.sql", &migrations_folder).await?;

    drop(container);
    Ok(())
}

#[tokio::test]
async fn test_dump_composite_primary_key_sqlite() -> Result<()> {
    let tmp_dir = TempDir::new()?;
    let db_file = tmp_dir.path().join("test_composite_pk.sqlite");
    File::create(&db_file)?;

    let database_url = format!("sqlite://{}", db_file.to_str().unwrap());
    let schema_file = "sqlite_composite_pk_schema.sql";
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();

    setup_composite_pk_schema(&database_url).await?;
    run_composite_pk_dump_test(&database_url, schema_file, &migrations_folder).await?;

    Ok(())
}

async fn setup_multi_schema_postgres(database_url: &str) -> Result<()> {
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

    let queries = vec![
        "CREATE SCHEMA authz;",
        r#"CREATE TABLE authz."group" (name TEXT PRIMARY KEY);"#,
        r#"CREATE SEQUENCE authz."test";"#,
        r#"CREATE TABLE public."group" (name TEXT PRIMARY KEY);"#,
        r#"CREATE SEQUENCE public."test";"#,
    ];

    for query in queries {
        client.execute(query, false).await?;
    }

    Ok(())
}

async fn run_multi_schema_dump_test(
    database_url: &str,
    schema_file: &str,
    migrations_folder: &str,
) -> Result<()> {
    dump(
        database_url.to_string(),
        None,
        "schema_migrations".to_string(),
        migrations_folder.to_string(),
        schema_file.to_string(),
        Some(30),
    )
    .await?;

    let schema_path = Path::new(migrations_folder).join(schema_file);
    assert!(schema_path.exists(), "Schema file should be created");

    let schema_content = fs::read_to_string(&schema_path)?;

    assert!(
        schema_content.contains("authz"),
        "Schema should contain 'authz' references, got:\n{}",
        schema_content
    );

    assert!(
        schema_content.contains("public"),
        "Schema should contain 'public' references, got:\n{}",
        schema_content
    );

    let table_lines: Vec<&str> = schema_content
        .lines()
        .filter(|l| l.contains("CREATE TABLE") && l.contains("\"group\""))
        .collect();

    assert_eq!(
        table_lines.len(),
        2,
        "Should have 2 CREATE TABLE lines for 'group' (one per schema), got {} lines:\n{}\nFull schema:\n{}",
        table_lines.len(),
        table_lines.join("\n"),
        schema_content
    );

    let sequence_lines: Vec<&str> = schema_content
        .lines()
        .filter(|l| l.contains("CREATE SEQUENCE") && l.contains("\"test\""))
        .collect();

    assert_eq!(
        sequence_lines.len(),
        2,
        "Should have 2 CREATE SEQUENCE lines for 'test' (one per schema), got {} lines:\n{}\nFull schema:\n{}",
        sequence_lines.len(),
        sequence_lines.join("\n"),
        schema_content
    );

    Ok(())
}

#[tokio::test]
async fn test_dump_multi_schema_postgres() -> Result<()> {
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

    let tmp_dir = TempDir::new()?;
    let migrations_folder = tmp_dir.path().to_str().unwrap().to_string();
    setup_multi_schema_postgres(&url).await?;
    run_multi_schema_dump_test(&url, "postgres_multi_schema.sql", &migrations_folder).await?;

    drop(container);
    Ok(())
}
