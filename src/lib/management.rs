use crate::database_drivers;
use anyhow::Result;

pub async fn create(
    database_url: String,
    db_token: Option<String>,
    wait_timeout: Option<usize>,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
) -> Result<()> {
    let mut database = database_drivers::new(
        database_url,
        db_token,
        migrations_table,
        migrations_folder,
        schema_file,
        wait_timeout,
        false,
    )
    .await?;

    database.create_database().await?;

    Ok(())
}

pub async fn drop(
    database_url: String,
    db_token: Option<String>,
    wait_timeout: Option<usize>,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
) -> Result<()> {
    let mut database = database_drivers::new(
        database_url,
        db_token,
        migrations_table,
        migrations_folder,
        schema_file,
        wait_timeout,
        false,
    )
    .await?;

    database.drop_database().await?;

    Ok(())
}
