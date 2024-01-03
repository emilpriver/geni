use std::path::PathBuf;

use crate::{
    config::{database_url, migration_folder},
    database_drivers,
    utils::{get_local_migrations, read_file_content},
};
use anyhow::{bail, Result};
use log::info;

pub async fn status(verbose: bool) -> Result<()> {
    let database_url = database_url()?;
    let mut database = database_drivers::new(&database_url, true).await?;
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
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
