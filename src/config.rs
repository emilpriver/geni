use anyhow::{bail, Result};
use clap::ArgMatches;
use std::env;

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("MIGRATION_DIR") {
        return v;
    }

    "./migrations".to_string()
}

pub fn database_url(query_matches: Option<&ArgMatches>) -> Result<&String> {
    if let Some(q) = query_matches {
        if let Some(v) = q.get_one::<String>("url") {
            return Ok(v);
        }
    }

    if let Ok(v) = env::var("DATABASE_URL") {
        return Ok(&v);
    }

    bail!("missing DATABASE_URL env variable")
}

pub fn database_token() -> Result<String> {
    if let Ok(v) = env::var("DATABASE_TOKEN") {
        return Ok(v);
    }

    bail!("missing DATABASE_TOKEN env variable")
}
