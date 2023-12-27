use crate::config::database_url;
use crate::database_drivers;
use anyhow::Result;

pub async fn create() -> Result<()> {
    let database_url = database_url()?;
    let mut database = database_drivers::new(&database_url, false).await?;

    database.create_database().await?;

    Ok(())
}

pub async fn drop() -> Result<()> {
    let database_url = database_url()?;
    let mut database = database_drivers::new(&database_url, false).await?;

    database.drop_database().await?;

    Ok(())
}
