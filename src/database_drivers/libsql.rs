use crate::database_drivers::DatabaseDriver;
use anyhow::{bail, Result};
use libsql_client::{de, Client, Config};
use std::future::Future;

pub struct LibSQLDriver {
    db: Client,
}

impl<'a> LibSQLDriver {
    pub async fn new<'b>(db_url: &str, token: &str) -> Result<LibSQLDriver> {
        let config = Config::new(db_url)?.with_auth_token(token);
        let client = match libsql_client::Client::from_config(config).await {
            Ok(c) => c,
            Err(err) => bail!("{:?}", err),
        };
        Ok(LibSQLDriver { db: client })
    }
}

impl<'a> DatabaseDriver<'a> for LibSQLDriver {
    fn execute(
        &'a self,
        query: &str,
    ) -> Box<dyn Future<Output = Result<(), anyhow::Error>> + Unpin + 'static> {
        Box::new(Box::pin(async move {
            self.db.execute(query).await?;
            Ok(())
        }))
    }

    fn get_or_create_schema_migrations(
        &self,
    ) -> Box<dyn Future<Output = Result<Vec<String>, anyhow::Error>> + Unpin + 'static> {
        Box::new(Box::pin(async move {
            let query = "CREATE TABLE IF NOT EXISTS schema_migrations (id TEXT PRIMARY KEY);";
            self.db.execute(query).await?;
            let query = "SELECT id FROM schema_migrations;";
            let result = self.db.execute(query).await?;
            result
                .rows
                .iter()
                .map(de::from_row)
                .collect::<Result<Vec<String>>>()
        }))
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use mockall::predicate::*;
    use mockall::*;

    // Mock the LibSQLDriver
    mock! {
        LibSQLDriver {
            fn execute(&self, query: &str) -> Box<dyn Future<Output = Result<()>> + 'static>;
            fn get_or_create_schema_migrations(&self) -> Box<dyn Future<Output = Result<Vec<String>>> + 'static>;
        }
    }

    #[test]
    fn test_execute() {
        let mut mock_driver = MockLibSQLDriver::new();
        mock_driver.expect_execute()
            .with(eq("SELECT * FROM test;"))
            .return_const(Box::pin(async { Ok(()) }));

        let result = block_on(mock_driver.execute("SELECT * FROM test;"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_or_create_schema_migrations() {
        let mut mock_driver = MockLibSQLDriver::new();
        mock_driver.expect_get_or_create_schema_migrations()
            .return_const(Box::pin(async { Ok(vec!["migration1".to_string(), "migration2".to_string()]) }));

        let result = block_on(mock_driver.get_or_create_schema_migrations());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec!["migration1".to_string(), "migration2".to_string()]);
    }
} */
