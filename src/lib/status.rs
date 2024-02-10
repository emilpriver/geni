use std::path::PathBuf;

use crate::{
    database_drivers,
    utils::{get_local_migrations, read_file_content},
};
use anyhow::{bail, Result};
use log::info;

pub async fn status(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    verbose: bool,
) -> Result<()> {
    let mut database = database_drivers::new(
        database_url,
        database_token,
        migration_table,
        migration_folder.clone(),
        schema_file,
        wait_timeout,
        true,
    )
    .await?;

    let path = PathBuf::from(&migration_folder);
    let files = match get_local_migrations(&path, "up") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    let migrations: Vec<String> = database
        .get_or_create_schema_migrations()
        .await?
        .into_iter()
        .map(|s| s.into_boxed_str())
        .collect::<Vec<Box<str>>>()
        .into_iter()
        .map(|s| s.into())
        .collect();

    for f in files {
        let id = Box::new(f.0.to_string());

        if !migrations.contains(&id) {
            if verbose {
                let query = read_file_content(&f.1);
                info!("Pending migration {}: \n {}", id, query);
            } else {
                info!("Pending {}", id);
            }
        }
    }

    Ok(())
}
