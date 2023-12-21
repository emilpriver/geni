use crate::database_drivers::DatabaseDriver;
use anyhow::Result;
use libsql_client::{de, local::Client};
use std::future::Future;
use std::pin::Pin;

use super::SchemaMigration;

pub struct SqliteDriver {
    db: Client,
}

impl<'a> SqliteDriver {
    pub async fn new<'b>(db_url: &str) -> Result<SqliteDriver> {
        let client = Client::new(db_url).unwrap();

        Ok(SqliteDriver { db: client })
    }
}

impl DatabaseDriver for SqliteDriver {
    fn execute<'a>(
        &'a mut self,
        query: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            self.db.execute(query)?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn get_or_create_schema_migrations(
        &mut self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + '_>> {
        let fut = async move {
            let query =
                "CREATE TABLE IF NOT EXISTS schema_migrations (id VARCHAR(255) PRIMARY KEY);";
            self.db.execute(query)?;
            let query = "SELECT id FROM schema_migrations ORDER BY id DESC;";
            let result = self.db.execute(query)?;

            let rows = result.rows.iter().map(de::from_row);

            let res = rows.collect::<Result<Vec<SchemaMigration>>>();

            res.map(|v| v.into_iter().map(|s| s.id).collect())
        };

        Box::pin(fut)
    }

    fn insert_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            self.db.execute(
                format!("INSERT INTO schema_migrations (id) VALUES ('{}');", id).as_str(),
            )?;
            Ok(())
        };

        Box::pin(fut)
    }

    fn remove_schema_migration<'a>(
        &'a mut self,
        id: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), anyhow::Error>> + '_>> {
        let fut = async move {
            self.db
                .execute(format!("DELETE FROM schema_migrations WHERE id = '{}';", id).as_str())?;
            Ok(())
        };

        Box::pin(fut)
    }
}
