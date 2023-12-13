use anyhow::{bail, Result};
use std::env;

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("MIGRATION_DIR") {
        return v;
    }

    "./migrations".to_string()
}

pub fn database_url() -> Result<String> {
    if let Ok(v) = env::var("DATABASE_URL") {
        return Ok(v);
    }

    bail!("missing DATABASE_URL env variable")
}
