use crate::database_drivers;
use anyhow::Result;

pub async fn dump(
    database_url: String,
    database_token: Option<String>,
    migrations_table: String,
    migrations_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
) -> Result<()> {
    let mut database = database_drivers::new(
        database_url,
        database_token,
        migrations_table,
        migrations_folder,
        schema_file,
        wait_timeout,
        true,
    )
    .await?;

    database.dump_database_schema().await?;

    Ok(())
}
