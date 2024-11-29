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
