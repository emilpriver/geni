use std::path::PathBuf;

use crate::{
    config::{database_url, migration_folder},
    database_drivers,
    utils::{get_local_migrations, read_file_content},
};
use anyhow::{bail, Result};
use log::info;

pub async fn status() -> Result<()> {
    let database_url = database_url()?;
    let mut database = database_drivers::new(&database_url, true).await?;
    let folder = migration_folder();

    let path = PathBuf::from(&folder);
    let files = match get_local_migrations(&path, "down") {
        Ok(f) => f,
        Err(err) => {
            bail!("Couldn't read migration folder: {:?}", err)
        }
    };

    let mut migrations = get_local_migrations(folder, ending).await?;
    migrations.sort_by(|a, b| a.version.cmp(&b.version));

    let mut applied_migrations = get_applied_migrations().await?;
    applied_migrations.sort_by(|a, b| a.version.cmp(&b.version));

    let mut migrations_to_apply = Vec::new();
    let mut migrations_to_revert = Vec::new();

    for migration in migrations {
        if applied_migrations.contains(&migration) {
            continue;
        }

        migrations_to_apply.push(migration);
    }

    for applied_migration in applied_migrations {
        if migrations.contains(&applied_migration) {
            continue;
        }

        migrations_to_revert.push(applied_migration);
    }

    if migrations_to_apply.is_empty() && migrations_to_revert.is_empty() {
        info!("No migrations to apply or revert");
        return Ok(());
    }

    if !migrations_to_apply.is_empty() {
        info!("Migrations to apply:");
        for migration in migrations_to_apply {
            info!("  {}", migration.name);
        }
    }

    if !migrations_to_revert.is_empty() {
        info!("Migrations to revert:");
        for migration in migrations_to_revert {
            info!("  {}", migration.name);
        }
    }

    Ok(())
}
