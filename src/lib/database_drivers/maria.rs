use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use log::info;
use sqlx::mysql::MySqlRow;
use sqlx::Executor;
use sqlx::{Connection, MySqlConnection, Row};
use std::future::Future;
use std::pin::Pin;

use super::utils;

pub struct MariaDBDriver {
    db: MySqlConnection,
    url: String,
    db_name: String,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
}

impl<'a> MariaDBDriver {
    pub async fn new<'b>(
        db_url: &str,
        database_name: &str,
        wait_timeout: Option<usize>,
        migrations_table: String,
        migrations_folder: String,
        schema_file: String,
    ) -> Result<MariaDBDriver> {
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

        let m = MariaDBDriver {
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

impl DatabaseDriver for MariaDBDriver {
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
                -- Maria SQL Schema dump automatic generated by geni
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
                            ) SEPARATOR', \n'
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
                    TABLE_SCHEMA = ?;
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
                        ) AS create_constraint_stmt
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
                        ) AS constraints
                    ORDER BY 
                        TABLE_NAME;
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
                        TABLE_NAME;
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
                    ) AS comments;
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
