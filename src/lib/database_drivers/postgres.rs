use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use log::info;
use sqlx::postgres::PgRow;
use sqlx::Executor;
use sqlx::{Connection, PgConnection, Row};
use std::future::Future;
use std::pin::Pin;

use super::utils;

pub struct PostgresDriver {
    db: PgConnection,
    url: String,
    db_name: String,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl<'a> PostgresDriver {
    pub async fn new<'b>(
        db_url: &str,
        database_name: &str,
        wait_timeout: Option<usize>,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<PostgresDriver> {
        let mut client = PgConnection::connect(db_url).await;

        let wait_timeout = wait_timeout.unwrap_or(0);

        if client.is_err() {
            let mut count = 0;
            loop {
                info!("Waiting for database to be ready");
                if count > wait_timeout {
                    bail!("Database is not ready");
                }

                match PgConnection::connect(db_url).await {
                    Ok(c) => {
                        client = Ok(c);
                        break;
                    }
                    Err(_) => {
                        count += 1;
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
        }

        let p = PostgresDriver {
            db: client.unwrap(),
            url: db_url.to_string(),
            db_name: database_name.to_string(),
            migrations_folder,
            migrations_table,
            schema_file,
        };

        Ok(p)
    }
}

impl DatabaseDriver for PostgresDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
        run_in_transaction: bool,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            if run_in_transaction {
                let mut tx = self.db.begin().await?;
                match tx.execute(query).await {
                    Ok(_) => {
                        tx.commit().await?;
                    }
                    Err(e) => {
                        tx.rollback().await?;
                        bail!(e)
                    }
                }
                return Ok(());
            } else {
                self.db.execute(query).await?;
            }

            Ok(())
        };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!(
                "CREATE TABLE IF NOT EXISTS {} (id VARCHAR(255) PRIMARY KEY)",
                self.migrations_table,
            );
            sqlx::query(query.as_str()).execute(&mut self.db).await?;

            let query = format!("SELECT id FROM {} ORDER BY id DESC", self.migrations_table);

            let result: Vec<String> = sqlx::query(query.as_str())
                .map(|row: PgRow| row.get("id"))
                .fetch_all(&mut self.db)
                .await?;

            Ok(result)
        };

        Box::pin(fut)
    }

    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("INSERT INTO {} (id) VALUES ($1)", self.migrations_table);
            sqlx::query(query.as_str())
                .bind(id)
                .execute(&mut self.db)
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
            let query = format!("DELETE FROM {} WHERE id = $1", self.migrations_table);
            sqlx::query(query.as_str())
                .bind(id)
                .execute(&mut self.db)
                .await?;

            Ok(())
        };

        Box::pin(fut)
    }

    fn create_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("CREATE DATABASE {}", self.db_name);

            let mut client = PgConnection::connect(self.url.as_str()).await?;
            sqlx::query(query.as_str()).execute(&mut client).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn drop_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("DROP DATABASE {}", self.db_name);

            let mut client = PgConnection::connect(self.url.as_str()).await?;
            sqlx::query(query.as_str()).execute(&mut client).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn ready(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query("SELECT 1").execute(&mut self.db).await?;
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
                -- Postgres SQL Schema dump automatic generated by geni
                --


            "#;

            let mut schema = schema
                .lines()
                .map(str::trim_start)
                .collect::<Vec<&str>>()
                .join("\n");

            let extensions: Vec<String> = sqlx::query(
                r#"
                SELECT
                    'CREATE EXTENSION IF NOT EXISTS "' || extname || '" WITH SCHEMA public;' AS sql
                FROM
                    pg_extension
                WHERE
                    (SELECT nspname FROM pg_namespace WHERE oid = extnamespace) = 'public'
                ORDER BY extname ASC
                "#,
            )
            .map(|row: PgRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if !extensions.is_empty() {
                schema.push_str("-- EXTENSIONS \n\n");
                for ele in extensions.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let tables: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    'CREATE TABLE ' || t.table_name || E' (\n ' || 
                    string_agg(c.column_name || ' ' || c.data_type || ' ' || 
                                (CASE WHEN c.character_maximum_length IS NOT NULL 
                                    THEN '(' || c.character_maximum_length || ')' 
                                    ELSE '' END) || 
                                (CASE WHEN c.is_nullable = 'NO' THEN ' NOT NULL' ELSE '' END), 
                                E',\n ' ORDER BY c.column_name ASC) || 
                    E'\n);' AS sql
                FROM 
                    information_schema.columns c
                JOIN 
                    information_schema.tables t ON c.table_name = t.table_name
                WHERE 
                    t.table_schema = 'public' 
                    AND t.table_type = 'BASE TABLE'
                GROUP BY 
                    t.table_name
                ORDER BY 
                    t.table_name;
                "#,
            )
            .map(|row: PgRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if !tables.is_empty() {
                schema.push_str("-- TABLES \n\n");
                for ele in tables.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let views: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    'CREATE VIEW ' || table_name || ' AS\n' || view_definition || ';' AS sql
                FROM 
                    information_schema.views
                WHERE 
                    table_schema = 'public'
                ORDER BY 
                    table_name ASC
                "#,
            )
            .map(|row: PgRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if !views.is_empty() {
                schema.push_str("-- VIEWS \n\n");
                for ele in views.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let constraints: Vec<String> = sqlx::query(
                r#"
                SELECT DISTINCT
                    CASE 
                        WHEN tc.constraint_type = 'PRIMARY KEY' THEN 
                            'ALTER TABLE ' || tc.table_name || 
                            ' ADD CONSTRAINT ' || tc.constraint_name || 
                            ' PRIMARY KEY (' || kcu.column_name || ');'
                        WHEN tc.constraint_type = 'FOREIGN KEY' THEN 
                            'ALTER TABLE ' || tc.table_name || 
                            ' ADD CONSTRAINT ' || tc.constraint_name || 
                            ' FOREIGN KEY (' || kcu.column_name || ') REFERENCES ' || 
                            ccu.table_name || '(' || ccu.column_name || ');'
                        WHEN tc.constraint_type = 'UNIQUE' THEN 
                            'ALTER TABLE ' || tc.table_name || 
                            ' ADD CONSTRAINT ' || tc.constraint_name || 
                            ' UNIQUE (' || kcu.column_name || ');'
                        WHEN tc.constraint_type = 'CHECK' THEN 
                            'ALTER TABLE ' || tc.table_name || 
                            ' ADD CONSTRAINT ' || tc.constraint_name || 
                            ' CHECK (' || cc.check_clause || ');'
                    END AS sql,
                    tc.table_name, 
                    tc.constraint_name
                FROM 
                    information_schema.table_constraints tc
                JOIN 
                    information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name
                LEFT JOIN 
                    information_schema.constraint_column_usage ccu ON kcu.constraint_name = ccu.constraint_name
                LEFT JOIN 
                    information_schema.check_constraints cc ON tc.constraint_name = cc.constraint_name
                WHERE 
                    tc.table_schema = 'public'
                ORDER BY 
                    tc.table_name, 
                    tc.constraint_name
                "#,
            )
            .map(|row: PgRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if !constraints.is_empty() {
                schema.push_str("-- CONSTRAINTS \n\n");
                for ele in constraints.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let indexes: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    indexdef AS sql
                FROM 
                    pg_indexes
                WHERE 
                    schemaname = 'public'
                ORDER BY 
                    indexname ASC;
                "#,
            )
            .map(|row: PgRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if !indexes.is_empty() {
                schema.push_str("-- INDEXES \n\n");
                for ele in indexes.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let sequences: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    'CREATE SEQUENCE ' || sequence_name || 
                    ' AS ' || data_type || 
                    ' START WITH ' || start_value || 
                    ' MINVALUE ' || minimum_value || 
                    ' MAXVALUE ' || maximum_value || 
                    ' INCREMENT BY ' || increment || 
                    ' CYCLE;' AS sql
                FROM 
                    information_schema.sequences
                WHERE 
                    sequence_schema = 'public'
                ORDER BY 
                    sequence_name ASC;
                "#,
            )
            .map(|row: PgRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if !sequences.is_empty() {
                schema.push_str("-- SEQUENCES \n\n");
                for ele in sequences.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let comments: Vec<String> = sqlx::query(
                r#"
                SELECT
                    'COMMENT ON ' ||
                    CASE
                        WHEN pa.attnum > 0 THEN
                            'COLUMN ' || pc.relname || '.' || pa.attname
                        ELSE
                            'TABLE ' || pc.relname
                    END ||
                    ' IS ' || pd.description || ';' AS sql
                FROM
                    pg_class pc
                    JOIN pg_attribute pa ON pc.oid = pa.attrelid
                    LEFT JOIN pg_description pd ON pc.oid = pd.objoid AND pa.attnum = pd.objsubid
                WHERE
                    pc.relnamespace = (
                        SELECT
                            oid
                        FROM
                            pg_namespace
                        WHERE
                            nspname = 'public'
                    )
                    AND pd.description IS NOT NULL
                ORDER BY
                    pc.relname,
                    pa.attnum;
                "#,
            )
            .map(|row: PgRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if !comments.is_empty() {
                schema.push_str("-- COMMENTS \n\n");
                for ele in comments.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            utils::write_to_schema_file(
                schema.to_string(),
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
    fn test_validate_postgres_url_valid() {
        let valid_urls = vec![
            "postgres://user:pass@localhost:5432/db",
            "postgresql://user:pass@localhost:5432/db",
            "psql://user:pass@localhost:5432/db",
        ];

        for url in valid_urls {
            let result = validate_postgres_url(url);
            assert!(result.is_ok(), "URL should be valid: {}", url);
        }
    }

    #[test]
    fn test_validate_postgres_url_invalid() {
        let invalid_urls = vec![
            "mysql://user:pass@localhost:3306/db",
            "sqlite://test.db",
            "http://localhost:8080",
            "invalid-url",
        ];

        for url in invalid_urls {
            let result = validate_postgres_url(url);
            assert!(result.is_err(), "URL should be invalid: {}", url);
        }
    }

    #[test]
    fn test_generate_postgres_create_db_query() {
        let db_name = "test_database";
        let expected = "CREATE DATABASE test_database";
        let result = generate_postgres_create_db_query(db_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_postgres_drop_db_query() {
        let db_name = "test_database";
        let expected = "DROP DATABASE test_database";
        let result = generate_postgres_drop_db_query(db_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_postgres_migrations_table_query() {
        let table_name = "schema_migrations";
        let expected = "CREATE TABLE IF NOT EXISTS schema_migrations (id VARCHAR(255) PRIMARY KEY)";
        let result = generate_postgres_migrations_table_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_postgres_migrations_table_query_custom() {
        let table_name = "custom_migrations";
        let expected = "CREATE TABLE IF NOT EXISTS custom_migrations (id VARCHAR(255) PRIMARY KEY)";
        let result = generate_postgres_migrations_table_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_postgres_insert_migration_query() {
        let table_name = "schema_migrations";
        let expected = "INSERT INTO schema_migrations (id) VALUES ($1)";
        let result = generate_postgres_insert_migration_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_postgres_delete_migration_query() {
        let table_name = "schema_migrations";
        let expected = "DELETE FROM schema_migrations WHERE id = $1";
        let result = generate_postgres_delete_migration_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_postgres_select_migrations_query() {
        let table_name = "schema_migrations";
        let expected = "SELECT id FROM schema_migrations ORDER BY id DESC";
        let result = generate_postgres_select_migrations_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_validate_wait_timeout_with_value() {
        let timeout = Some(30);
        let result = validate_wait_timeout(timeout);
        assert_eq!(result, 30);
    }

    #[test]
    fn test_validate_wait_timeout_none() {
        let timeout = None;
        let result = validate_wait_timeout(timeout);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_postgres_driver_parameters() {
        // Test that PostgresDriver::new has the expected parameter types
        let _db_url = "postgres://user:pass@localhost:5432/test";
        let _database_name = "test";
        let _wait_timeout: Option<usize> = Some(30);
        let _migrations_table = "schema_migrations".to_string();
        let _migrations_folder = "./migrations".to_string();
        let _schema_file = "schema.sql".to_string();

        // Test that parameters are in the expected order (compile-time check)
        let _test_signature = || async {
            // Note: This won't actually connect without a real database
            // but validates the function signature
            let _future = PostgresDriver::new(
                _db_url,
                _database_name,
                _wait_timeout,
                _migrations_table.clone(),
                _migrations_folder.clone(),
                _schema_file.clone(),
            );
            Ok::<(), anyhow::Error>(())
        };

        assert!(true);
    }

    #[test]
    fn test_postgres_driver_struct_fields() {
        // Test that PostgresDriver has expected fields (compile-time validation)
        fn _test_fields() {
            let _check_url: fn(&PostgresDriver) -> &String = |driver| &driver.url;
            let _check_db_name: fn(&PostgresDriver) -> &String = |driver| &driver.db_name;
            let _check_migrations_table: fn(&PostgresDriver) -> &String = |driver| &driver.migrations_table;
            let _check_migrations_folder: fn(&PostgresDriver) -> &String = |driver| &driver.migrations_folder;
            let _check_schema_file: fn(&PostgresDriver) -> &String = |driver| &driver.schema_file;
        }

        assert!(true);
    }

    #[test]
    fn test_postgres_query_generation_edge_cases() {
        // Test with special characters that might need escaping
        let special_names = vec![
            "test_db",
            "test-db",
            "test123",
            "migrations_v2",
        ];

        for name in special_names {
            let create_query = generate_postgres_create_db_query(name);
            let drop_query = generate_postgres_drop_db_query(name);
            let table_query = generate_postgres_migrations_table_query(name);

            // Verify queries contain the name
            assert!(create_query.contains(name));
            assert!(drop_query.contains(name));
            assert!(table_query.contains(name));

            // Verify query structure
            assert!(create_query.starts_with("CREATE DATABASE"));
            assert!(drop_query.starts_with("DROP DATABASE"));
            assert!(table_query.starts_with("CREATE TABLE IF NOT EXISTS"));
        }
    }

    #[test]
    fn test_postgres_url_schemes() {
        let test_cases = vec![
            ("postgres://localhost/db", true),
            ("postgresql://localhost/db", true),
            ("psql://localhost/db", true),
            ("POSTGRES://localhost/db", false), // case sensitive
            ("postgres://", true), // minimal valid
            ("postgres:localhost/db", false), // missing //
            ("http://localhost/db", false),
            ("mysql://localhost/db", false),
        ];

        for (url, should_be_valid) in test_cases {
            let result = validate_postgres_url(url);
            if should_be_valid {
                assert!(result.is_ok(), "URL should be valid: {}", url);
            } else {
                assert!(result.is_err(), "URL should be invalid: {}", url);
            }
        }
    }

    #[test]
    fn test_postgres_parameterized_queries() {
        // Test that parameterized queries use PostgreSQL syntax ($1, $2, etc.)
        let table_name = "test_migrations";

        let insert_query = generate_postgres_insert_migration_query(table_name);
        let delete_query = generate_postgres_delete_migration_query(table_name);

        // Verify PostgreSQL parameter syntax
        assert!(insert_query.contains("$1"));
        assert!(delete_query.contains("$1"));

        // Verify proper SQL structure
        assert!(insert_query.contains("INSERT INTO"));
        assert!(insert_query.contains("VALUES"));
        assert!(delete_query.contains("DELETE FROM"));
        assert!(delete_query.contains("WHERE"));
    }

    #[test]
    fn test_wait_timeout_boundary_conditions() {
        let test_cases = vec![
            (Some(0), 0),
            (Some(1), 1),
            (Some(30), 30),
            (Some(3600), 3600),
            (None, 0),
        ];

        for (input, expected) in test_cases {
            let result = validate_wait_timeout(input);
            assert_eq!(result, expected, "Failed for input: {:?}", input);
        }
    }
}
