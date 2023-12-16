use crate::database_drivers::DatabaseDriver;
use anyhow::Result;
use async_trait::async_trait;

pub struct PostgresDriver {}

impl PostgresDriver {
    pub async fn new(_db_url: &str) -> Result<PostgresDriver> {
        Ok(PostgresDriver {})
    }
}

#[async_trait]
impl DatabaseDriver for PostgresDriver {
    // execute query with the provided database
    async fn execute(self, query: &str) -> Result<()> {
        Ok(())
    }

    // execute query with the provided database
    async fn get_or_create_schema_migrations(self) -> Result<Vec<String>> {
        Ok(vec![])
    }
}
