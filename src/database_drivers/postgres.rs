use crate::database_drivers::DatabaseDriver;
use anyhow::Result;
use sqlx::postgres::PgRow;
use sqlx::{Connection, PgConnection, Row};
use std::future::Future;
use std::pin::Pin;

pub struct PostgresDriver {
    db: PgConnection,
}

impl<'a> PostgresDriver {
    pub async fn new<'b>(db_url: &str) -> Result<PostgresDriver> {
        let client = PgConnection::connect(db_url).await?;
        Ok(PostgresDriver { db: client })
    }
}

impl DatabaseDriver for PostgresDriver {
    fn execute<'a>(
        &'a self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query(query).execute(&mut self.db).await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move {
            let query = "CREATE TABLE IF NOT EXISTS schema_migrations (id TEXT PRIMARY KEY);";
            sqlx::query(query).execute(&mut self.db).await?;
            let query = "SELECT id FROM schema_migrations ORDER BY id DESC;";
            let result: Vec<String> = sqlx::query(query)
                .map(|row: PgRow| row.get("id"))
                .fetch_all(&mut self.db)
                .await?;

            Ok(result)
        };

        Box::pin(fut)
    }

    fn insert_schema_migration<'a>(
        &'a self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query("INSERT INTO schema_migrations (id) VALUES ('?');")
                .bind(id)
                .execute(&mut self.db)
                .await?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn remove_schema_migration<'a>(
        &'a self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            sqlx::query("delete from schema_migrations where id = ?;")
                .bind(id)
                .execute(&mut self.db)
                .await?;

            Ok(())
        };

        Box::pin(fut)
    }
}
