use crate::database_drivers::{utils, DatabaseDriver};
use anyhow::{bail, Result};
use log::{error, info};
use regex::Regex;
use sqlx::mysql::MySqlRow;
use sqlx::Executor;
use sqlx::{Connection, MySqlConnection, Row};
use std::future::Future;
use std::pin::Pin;
use tokio::process::Command;
use url::Url;

pub struct MySQLDriver {
    db: MySqlConnection,
    url: String,
    db_name: String,
    url_path: Url,
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
            url_path,
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
                        error!("Error executing query: {}", e);
                        tx.rollback().await?;
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
                -- SQL Schema dump automatic generated by geni
                --


                -- TABLES
            "#;

            let mut schema = schema
                .lines()
                .map(str::trim_start)
                .collect::<Vec<&str>>()
                .join("\n");

            let tables: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    CONCAT('CREATE TABLE ', t.table_name, ' (\n', 
                        GROUP_CONCAT(CONCAT('  ', c.column_name, ' ', c.column_type, 
                                            IF(c.is_nullable = 'NO', ' NOT NULL', ''), 
                                            IF(c.column_default IS NOT NULL, CONCAT(' DEFAULT ', c.column_default), '')) 
                        SEPARATOR ',\n'), 
                    '\n);') AS sql
                FROM 
                    information_schema.columns c
                JOIN 
                    information_schema.tables t ON c.table_name = t.table_name
                WHERE 
                    t.table_schema = DATABASE() 
                    AND t.table_type = 'BASE TABLE'
                GROUP BY 
                    t.table_name
                ORDER BY 
                    t.table_name;
                "#,
                )
                .map(|row: MySqlRow| row.get("sql"))
                .fetch_all(&mut self.db)
                .await?;

            if tables.len() > 0 {
                schema.push_str("-- TABLES \n\n");
                for ele in tables.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let views: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    CONCAT('CREATE VIEW ', table_name, ' AS\n', view_definition, ';') AS sql
                FROM 
                    information_schema.views
                WHERE 
                    table_schema = DATABASE()
                ORDER BY 
                    table_name;
                "#,
            )
            .map(|row: MySqlRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if views.len() > 0 {
                schema.push_str("-- VIEWS \n\n");
                for ele in views.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let constraints: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    CASE 
                        WHEN tc.constraint_type = 'PRIMARY KEY' THEN 
                            CONCAT('ALTER TABLE ', tc.table_name, 
                                ' ADD CONSTRAINT ', tc.constraint_name, 
                                ' PRIMARY KEY (', GROUP_CONCAT(kcu.column_name ORDER BY kcu.ordinal_position SEPARATOR ', '), ');')
                        WHEN tc.constraint_type = 'FOREIGN KEY' THEN 
                            CONCAT('ALTER TABLE ', tc.table_name, 
                                ' ADD CONSTRAINT ', tc.constraint_name, 
                                ' FOREIGN KEY (', GROUP_CONCAT(kcu.column_name ORDER BY kcu.ordinal_position SEPARATOR ', '), ') REFERENCES ', 
                                ccu.table_name, '(', GROUP_CONCAT(ccu.column_name ORDER BY ccu.ordinal_position SEPARATOR ', '), ');')
                        WHEN tc.constraint_type = 'UNIQUE' THEN 
                            CONCAT('ALTER TABLE ', tc.table_name, 
                                ' ADD CONSTRAINT ', tc.constraint_name, 
                                ' UNIQUE (', GROUP_CONCAT(kcu.column_name ORDER BY kcu.ordinal_position SEPARATOR ', '), ');')
                        WHEN tc.constraint_type = 'CHECK' THEN 
                            CONCAT('ALTER TABLE ', tc.table_name, 
                                ' ADD CONSTRAINT ', tc.constraint_name, 
                                ' CHECK (', cc.check_clause, ');')
                    END AS sql
                FROM 
                    information_schema.table_constraints tc
                JOIN 
                    information_schema.key_column_usage kcu ON tc.constraint_name = kcu.constraint_name
                LEFT JOIN 
                    information_schema.referential_constraints rc ON tc.constraint_name = rc.constraint_name
                LEFT JOIN 
                    information_schema.constraint_column_usage ccu ON rc.unique_constraint_name = ccu.constraint_name
                LEFT JOIN 
                    information_schema.check_constraints cc ON tc.constraint_name = cc.constraint_name
                WHERE 
                    tc.table_schema = DATABASE()
                GROUP BY 
                    tc.constraint_type, tc.table_name, tc.constraint_name, cc.check_clause
                ORDER BY 
                    tc.table_name, tc.constraint_name;
                "#,
            )
            .map(|row: MySqlRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if constraints.len() > 0 {
                schema.push_str("-- CONSTRAINTS \n\n");
                for ele in constraints.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let indexes: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    index_definition AS sql
                FROM 
                    information_schema.statistics
                WHERE 
                    table_schema = DATABASE()
                ORDER BY 
                    index_name;
                "#,
            )
            .map(|row: MySqlRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if indexes.len() > 0 {
                schema.push_str("-- INDEXES \n\n");
                for ele in indexes.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let sequences: Vec<String> = sqlx::query(
                r#"
                SELECT 
                    CONCAT('CREATE SEQUENCE ', sequence_name, 
                        ' START WITH ', start_value, 
                        ' INCREMENT BY ', increment_by, 
                        ' MINVALUE ', min_value, 
                        ' MAXVALUE ', max_value, 
                        ' CYCLE ', IF(cycle_option = 'YES', 'YES', 'NO'), ';') AS sql
                FROM 
                    information_schema.sequences
                WHERE 
                    sequence_schema = DATABASE()
                ORDER BY 
                    sequence_name;
                "#,
            )
            .map(|row: MySqlRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if sequences.len() > 0 {
                schema.push_str("-- SEQUENCES \n\n");
                for ele in sequences.iter() {
                    schema.push_str(ele.as_str());
                    schema.push_str("\n\n")
                }
            }

            let comments: Vec<String> = sqlx::query(
                r#"
                SELECT
                    CONCAT('COMMENT ON ',
                        CASE
                            WHEN c.column_name IS NOT NULL THEN
                                CONCAT('COLUMN ', t.table_name, '.', c.column_name)
                            ELSE
                                CONCAT('TABLE ', t.table_name)
                        END,
                        ' IS ', c.column_comment, ';') AS sql
                FROM
                    information_schema.tables t
                LEFT JOIN
                    information_schema.columns c ON t.table_name = c.table_name AND t.table_schema = c.table_schema
                WHERE
                    t.table_schema = DATABASE()
                    AND (c.column_comment IS NOT NULL OR t.table_comment IS NOT NULL)
                ORDER BY
                    t.table_name, c.ordinal_position;
                "#,
            )
            .map(|row: MySqlRow| row.get("sql"))
            .fetch_all(&mut self.db)
            .await?;

            if comments.len() > 0 {
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
