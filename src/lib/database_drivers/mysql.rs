use crate::database_drivers::{utils, DatabaseDriver};
use anyhow::{bail, Result};
use log::info;

use sqlx::mysql::MySqlRow;
use sqlx::Executor;
use sqlx::{Connection, MySqlConnection, Row};
use std::future::Future;
use std::pin::Pin;

pub struct MySQLDriver {
    db: MySqlConnection,
    url: String,
    db_name: String,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl<'a> MySQLDriver {
    pub async fn new<'b>(
        db_url: &str,
        database_name: &str,
        wait_timeout: Option<usize>,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<MySQLDriver> {
        let mut client = MySqlConnection::connect(db_url).await;

        let wait_timeout = wait_timeout.unwrap_or(0);

        if client.is_err() {
            let mut count = 0;
            loop {
                info!("Waiting for database to be ready");
                if count > wait_timeout {
                    bail!("Database is not ready");
                }

                match MySqlConnection::connect(db_url).await {
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

        let mut url_path = url::Url::parse(db_url)?;
        if url_path.host_str().unwrap() == "localhost" {
            url_path.set_host(Some("127.0.0.1"))?;
        }

        let m = MySQLDriver {
            db: client.unwrap(),
            url: db_url.to_string(),
            db_name: database_name.to_string(),
            migrations_folder,
            migrations_table,
            schema_file,
        };

        Ok(m)
    }
}

impl DatabaseDriver for MySQLDriver {
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
                        bail!(e);
                    }
                }
                return Ok(());
            }

            self.db.execute(query).await?;

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
                .map(|row: MySqlRow| row.get("id"))
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
            let query = format!("INSERT INTO {} (id) VALUES (?)", self.migrations_table);
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
            let query = format!("DELETE FROM {} WHERE id = ?", self.migrations_table);
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
            let query = format!("CREATE DATABASE IF NOT EXISTS {}", self.db_name);

            let mut client = MySqlConnection::connect(self.url.as_str()).await?;
            sqlx::query(query.as_str()).execute(&mut client).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn drop_database(&mut self) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            let query = format!("DROP DATABASE IF EXISTS {}", self.db_name);

            let mut client = MySqlConnection::connect(self.url.as_str()).await?;
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
                -- MySQL SQL Schema dump automatic generated by geni
                --


            "#;

            let mut schema = schema
                .lines()
                .map(str::trim_start)
                .collect::<Vec<&str>>()
                .join("\n");

            let tables: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    CONCAT(
                        'CREATE TABLE ', 
                        TABLE_NAME, 
                        ' (\n',
                        GROUP_CONCAT(
                            CONCAT(
                                '  ', COLUMN_NAME, ' ', COLUMN_TYPE,
                                IF(IS_NULLABLE = 'NO', ' NOT NULL', ''),
                                IF(COLUMN_DEFAULT IS NOT NULL, CONCAT(' DEFAULT ', COLUMN_DEFAULT), '')
                            ) 
                            ORDER BY COLUMN_NAME ASC
                            SEPARATOR', \n'
                        ),
                        '\n);'
                    ) AS create_table_stmt
                FROM 
                    INFORMATION_SCHEMA.COLUMNS
                WHERE 
                    TABLE_SCHEMA = ? AND TABLE_NAME NOT IN (SELECT TABLE_NAME FROM INFORMATION_SCHEMA.VIEWS WHERE TABLE_SCHEMA = ?)
                GROUP BY 
                    TABLE_NAME
                ORDER BY 
                    TABLE_NAME;
                "#,
            )
            .bind(&self.db_name)
            .bind(&self.db_name)
            .map(|row: MySqlRow| row.get("create_table_stmt"))
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
                    CONCAT(
                        'CREATE VIEW ', 
                        TABLE_NAME, 
                        ' AS ', 
                        VIEW_DEFINITION, 
                        ';'
                    ) AS create_view_stmt
                FROM 
                    INFORMATION_SCHEMA.VIEWS
                WHERE 
                    TABLE_SCHEMA = ?
                ORDER BY TABLE_SCHEMA asc
                "#,
            )
            .bind(&self.db_name)
            .map(|row: MySqlRow| row.get("create_view_stmt"))
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
                        CONCAT(
                            'ALTER TABLE ', 
                            TABLE_NAME, 
                            ' ADD CONSTRAINT ',
                            CASE 
                                WHEN CONSTRAINT_NAME = 'PRIMARY' THEN 'PRIMARY KEY'
                                WHEN INDEX_NAME != 'PRIMARY' THEN 'UNIQUE'
                                ELSE 'FOREIGN KEY'
                            END, 
                            ' (', 
                            COLUMN_NAME, 
                            CASE 
                                WHEN REFERENCED_TABLE_NAME IS NOT NULL THEN 
                                    CONCAT(') REFERENCES ', REFERENCED_TABLE_NAME, ' (', REFERENCED_COLUMN_NAME, ')')
                                ELSE ')'
                            END, 
                            ';'
                        ) AS create_constraint_stmt,
                        TABLE_NAME
                    FROM 
                        (
                        SELECT 
                            TABLE_NAME, 
                            COLUMN_NAME, 
                            CONSTRAINT_NAME, 
                            NULL AS INDEX_NAME, 
                            NULL AS REFERENCED_TABLE_NAME, 
                            NULL AS REFERENCED_COLUMN_NAME
                        FROM 
                            INFORMATION_SCHEMA.KEY_COLUMN_USAGE
                        WHERE 
                            TABLE_SCHEMA = ? 
                            AND CONSTRAINT_NAME = 'PRIMARY'
                        UNION ALL
                        SELECT 
                            TABLE_NAME, 
                            COLUMN_NAME, 
                            NULL AS CONSTRAINT_NAME, 
                            INDEX_NAME, 
                            NULL AS REFERENCED_TABLE_NAME, 
                            NULL AS REFERENCED_COLUMN_NAME
                        FROM 
                            INFORMATION_SCHEMA.STATISTICS
                        WHERE 
                            TABLE_SCHEMA = ? 
                            AND INDEX_NAME != 'PRIMARY'
                        UNION ALL
                        SELECT 
                            TABLE_NAME, 
                            COLUMN_NAME, 
                            CONSTRAINT_NAME, 
                            NULL AS INDEX_NAME, 
                            REFERENCED_TABLE_NAME, 
                            REFERENCED_COLUMN_NAME
                        FROM 
                            INFORMATION_SCHEMA.KEY_COLUMN_USAGE
                        WHERE 
                            TABLE_SCHEMA = ? 
                            AND REFERENCED_TABLE_NAME IS NOT NULL
                        ORDER BY COLUMN_NAME asc
                        ) AS constraints
                    ORDER BY 
                        TABLE_NAME asc
                "#,
                )
                .bind(&self.db_name)
                .bind(&self.db_name)
                .bind(&self.db_name)
                .map(|row: MySqlRow| row.get("create_constraint_stmt"))
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
                        CONCAT(
                            'CREATE INDEX ', 
                            INDEX_NAME, 
                            ' ON ', 
                            TABLE_NAME, 
                            ' (', 
                            COLUMN_NAME, 
                            ');'
                        ) AS create_index_stmt
                    FROM 
                        INFORMATION_SCHEMA.STATISTICS
                    WHERE 
                        TABLE_SCHEMA = ?
                    GROUP BY 
                        TABLE_NAME, INDEX_NAME, COLUMN_NAME
                    ORDER BY 
                        TABLE_NAME, COLUMN_NAME asc
                "#,
            )
            .bind(&self.db_name)
            .map(|row: MySqlRow| row.get("create_index_stmt"))
            .fetch_all(&mut self.db)
            .await?;

            if !indexes.is_empty() {
                schema.push_str("-- INDEXES \n\n");
                for ele in indexes.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let comments: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    CONCAT(
                        CASE 
                            WHEN TABLE_COMMENT IS NOT NULL THEN 
                                CONCAT('ALTER TABLE ', TABLE_NAME, ' COMMENT = ''', TABLE_COMMENT, ''';')
                            ELSE 
                                CONCAT('ALTER TABLE ', TABLE_NAME, ' MODIFY COLUMN ', COLUMN_NAME, ' COMMENT ''', COLUMN_COMMENT, ''';')
                        END
                    ) AS comment_stmt
                FROM 
                    (
                        SELECT TABLE_NAME, TABLE_COMMENT, NULL AS COLUMN_NAME, NULL AS COLUMN_COMMENT
                        FROM INFORMATION_SCHEMA.TABLES
                        WHERE TABLE_SCHEMA = ? AND (TABLE_COMMENT IS NOT NULL OR TABLE_COMMENT != '')
                        UNION ALL
                        SELECT TABLE_NAME, NULL, COLUMN_NAME, COLUMN_COMMENT
                        FROM INFORMATION_SCHEMA.COLUMNS
                        WHERE TABLE_SCHEMA = ? AND (COLUMN_COMMENT IS NOT NULL OR COLUMN_COMMENT != '')
                    ) AS comments
                ORDER BY TABLE_NAME, COLUMN_NAME
                "#,
            )
            .bind(&self.db_name)
            .bind(&self.db_name)
            .map(|row: MySqlRow| row.get("comment_stmt"))
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
    fn test_validate_mysql_url_valid() {
        let valid_urls = vec![
            "mysql://user:pass@localhost:3306/db",
            "mysql://root@localhost:3306/test",
            "mysql://user@127.0.0.1:3306/database",
        ];

        for url in valid_urls {
            let result = validate_mysql_url(url);
            assert!(result.is_ok(), "URL should be valid: {}", url);
        }
    }

    #[test]
    fn test_validate_mysql_url_invalid() {
        let invalid_urls = vec![
            "postgres://user:pass@localhost:5432/db",
            "sqlite://test.db",
            "http://localhost:8080",
            "mariadb://user@localhost/db",
            "invalid-url",
        ];

        for url in invalid_urls {
            let result = validate_mysql_url(url);
            assert!(result.is_err(), "URL should be invalid: {}", url);
        }
    }

    #[test]
    fn test_generate_mysql_create_db_query() {
        let db_name = "test_database";
        let expected = "CREATE DATABASE IF NOT EXISTS test_database";
        let result = generate_mysql_create_db_query(db_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_mysql_drop_db_query() {
        let db_name = "test_database";
        let expected = "DROP DATABASE IF EXISTS test_database";
        let result = generate_mysql_drop_db_query(db_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_mysql_migrations_table_query() {
        let table_name = "schema_migrations";
        let expected = "CREATE TABLE IF NOT EXISTS schema_migrations (id VARCHAR(255) PRIMARY KEY)";
        let result = generate_mysql_migrations_table_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_mysql_insert_migration_query() {
        let table_name = "schema_migrations";
        let expected = "INSERT INTO schema_migrations (id) VALUES (?)";
        let result = generate_mysql_insert_migration_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_mysql_delete_migration_query() {
        let table_name = "schema_migrations";
        let expected = "DELETE FROM schema_migrations WHERE id = ?";
        let result = generate_mysql_delete_migration_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_generate_mysql_select_migrations_query() {
        let table_name = "schema_migrations";
        let expected = "SELECT id FROM schema_migrations ORDER BY id DESC";
        let result = generate_mysql_select_migrations_query(table_name);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_normalize_mysql_localhost_url() {
        let localhost_url = "mysql://user:pass@localhost:3306/db";
        let result = normalize_mysql_localhost_url(localhost_url).unwrap();
        assert_eq!(result, "mysql://user:pass@127.0.0.1:3306/db");
    }

    #[test]
    fn test_normalize_mysql_localhost_url_already_ip() {
        let ip_url = "mysql://user:pass@127.0.0.1:3306/db";
        let result = normalize_mysql_localhost_url(ip_url).unwrap();
        assert_eq!(result, "mysql://user:pass@127.0.0.1:3306/db");
    }

    #[test]
    fn test_normalize_mysql_localhost_url_other_host() {
        let other_url = "mysql://user:pass@remote.server.com:3306/db";
        let result = normalize_mysql_localhost_url(other_url).unwrap();
        assert_eq!(result, "mysql://user:pass@remote.server.com:3306/db");
    }

    #[test]
    fn test_mysql_driver_parameters() {
        // Test that MySQLDriver::new has the expected parameter types
        let _db_url = "mysql://user:pass@localhost:3306/test";
        let _database_name = "test";
        let _wait_timeout: Option<usize> = Some(30);
        let _migrations_table = "schema_migrations".to_string();
        let _migrations_folder = "./migrations".to_string();
        let _schema_file = "schema.sql".to_string();

        // Test that parameters are in the expected order (compile-time check)
        let _test_signature = || async {
            // Note: This won't actually connect without a real database
            // but validates the function signature
            let _future = MySQLDriver::new(
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
    fn test_mysql_driver_struct_fields() {
        // Test that MySQLDriver has expected fields (compile-time validation)
        fn _test_fields() {
            let _check_url: fn(&MySQLDriver) -> &String = |driver| &driver.url;
            let _check_db_name: fn(&MySQLDriver) -> &String = |driver| &driver.db_name;
            let _check_migrations_table: fn(&MySQLDriver) -> &String = |driver| &driver.migrations_table;
            let _check_migrations_folder: fn(&MySQLDriver) -> &String = |driver| &driver.migrations_folder;
            let _check_schema_file: fn(&MySQLDriver) -> &String = |driver| &driver.schema_file;
        }

        assert!(true);
    }

    #[test]
    fn test_mysql_vs_postgres_query_differences() {
        // Test that MySQL queries differ from PostgreSQL in parameter syntax
        let table_name = "test_migrations";

        let mysql_insert = generate_mysql_insert_migration_query(table_name);
        let mysql_delete = generate_mysql_delete_migration_query(table_name);

        let postgres_insert = generate_postgres_insert_migration_query(table_name);
        let postgres_delete = generate_postgres_delete_migration_query(table_name);

        // MySQL uses ? parameters, PostgreSQL uses $1 parameters
        assert!(mysql_insert.contains("?"));
        assert!(mysql_delete.contains("?"));
        assert!(postgres_insert.contains("$1"));
        assert!(postgres_delete.contains("$1"));

        // Both should have same basic structure but different parameter syntax
        assert!(mysql_insert.contains("INSERT INTO"));
        assert!(mysql_delete.contains("DELETE FROM"));
    }

    #[test]
    fn test_mysql_database_operation_queries() {
        let db_name = "test_db";

        let create_query = generate_mysql_create_db_query(db_name);
        let drop_query = generate_mysql_drop_db_query(db_name);

        // MySQL includes IF NOT EXISTS / IF EXISTS clauses
        assert!(create_query.contains("IF NOT EXISTS"));
        assert!(drop_query.contains("IF EXISTS"));

        // Verify basic structure
        assert!(create_query.starts_with("CREATE DATABASE"));
        assert!(drop_query.starts_with("DROP DATABASE"));
    }

    #[test]
    fn test_mysql_url_scheme_validation() {
        let test_cases = vec![
            ("mysql://localhost/db", true),
            ("mysql://user@localhost/db", true),
            ("mysql://user:pass@localhost:3306/db", true),
            ("MySQL://localhost/db", false), // case sensitive
            ("mysql://", true), // minimal valid
            ("mysql:localhost/db", false), // missing //
            ("postgres://localhost/db", false),
            ("mariadb://localhost/db", false),
        ];

        for (url, should_be_valid) in test_cases {
            let result = validate_mysql_url(url);
            if should_be_valid {
                assert!(result.is_ok(), "URL should be valid: {}", url);
            } else {
                assert!(result.is_err(), "URL should be invalid: {}", url);
            }
        }
    }

    #[test]
    fn test_mysql_special_database_names() {
        let special_names = vec![
            "test_db",
            "test-db",
            "test123",
            "migrations_v2",
            "TestDB",
        ];

        for name in special_names {
            let create_query = generate_mysql_create_db_query(name);
            let drop_query = generate_mysql_drop_db_query(name);
            let table_query = generate_mysql_migrations_table_query(name);

            // Verify queries contain the name
            assert!(create_query.contains(name));
            assert!(drop_query.contains(name));
            assert!(table_query.contains(name));

            // Verify MySQL-specific features
            assert!(create_query.contains("IF NOT EXISTS"));
            assert!(drop_query.contains("IF EXISTS"));
        }
    }

    #[test]
    fn test_mysql_url_host_normalization_edge_cases() {
        let test_cases = vec![
            ("mysql://localhost:3306/db", "mysql://127.0.0.1:3306/db"),
            ("mysql://localhost/db", "mysql://127.0.0.1/db"),
            ("mysql://user@localhost:3306/db", "mysql://user@127.0.0.1:3306/db"),
            ("mysql://user:pass@localhost:3306/db", "mysql://user:pass@127.0.0.1:3306/db"),
            ("mysql://127.0.0.1:3306/db", "mysql://127.0.0.1:3306/db"), // unchanged
            ("mysql://192.168.1.100:3306/db", "mysql://192.168.1.100:3306/db"), // unchanged
            ("mysql://example.com:3306/db", "mysql://example.com:3306/db"), // unchanged
        ];

        for (input, expected) in test_cases {
            let result = normalize_mysql_localhost_url(input).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }
}
