use crate::database_drivers;
use anyhow::Result;

pub async fn create(database_url: &String, db_token: Option<String>) -> Result<()> {
    let mut database = database_drivers::new(database_url, db_token, false).await?;

    database.create_database().await?;

    Ok(())
}

pub async fn drop(database_url: &String, db_token: Option<String>) -> Result<()> {
    let mut database = database_drivers::new(database_url, db_token, false).await?;

    database.drop_database().await?;

    Ok(())
}
