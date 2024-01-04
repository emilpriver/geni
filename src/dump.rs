use crate::{config::database_url, database_drivers};
use anyhow::Result;

pub async fn dump() -> Result<()> {
    let database_url = database_url()?;
    let mut database = database_drivers::new(&database_url, true).await?;

    database.dump_database_schema().await?;

    Ok(())
}
