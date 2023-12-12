use anyhow::Result;
use chrono::Utc;

pub fn generate_new_migration(migration_name: &str) -> Result<&str> {
    let timestamp = Utc::now().timestamp();
    let name = migration_name.replace(" ", "_").to_lowercase();

    Ok(migration_name)
}
