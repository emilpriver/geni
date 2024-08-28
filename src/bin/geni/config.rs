use anyhow::{bail, Result};
use std::{env, fs, path::Path};

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("DATABASE_MIGRATIONS_FOLDER") {
        if !v.is_empty() {
            return v;
        }
    }

    "./migrations".to_string()
}

pub fn database_url() -> Result<String> {
    if let Ok(v) = env::var("DATABASE_URL") {
        if !v.is_empty() {
            return Ok(v);
        }
    }

    bail!("missing DATABASE_URL env variable")
}

pub fn database_token() -> Option<String> {
    if let Ok(v) = env::var("DATABASE_TOKEN") {
        if !v.is_empty() {
            return Some(v);
        }
    }

    None
}

pub fn wait_timeout() -> usize {
    if let Ok(v) = env::var("DATABASE_WAIT_TIMEOUT") {
        if !v.is_empty() {
            if let Ok(v) = v.parse::<usize>() {
                return v;
            }
        }
    }

    30
}

pub fn dump_schema_file() -> bool {
    if let Ok(v) = env::var("DATABASE_NO_DUMP_SCHEMA") {
        if v == "true" {
            return false;
        }
    }

    true
}

pub fn schema_file() -> String {
    if let Ok(v) = env::var("DATABASE_SCHEMA_FILE") {
        if !v.is_empty() {
            return v;
        }
    }

    "schema.sql".to_string()
}

pub fn migrations_table() -> String {
    if let Ok(v) = env::var("DATABASE_MIGRATIONS_TABLE") {
        if !v.is_empty() {
            return v;
        }
    }

    "schema_migrations".to_string()
}

pub struct GeniConfig {
    pub database_url: String,
    pub database_token: Option<String>,
}

pub fn load_config_file(project_name: &str) -> Result<GeniConfig> {
    let file_path = "./geni.toml";
    if Path::new(file_path).try_exists().is_err() {
        bail!("{} don't exist", file_path);
    }

    let contents = fs::read_to_string(file_path)?;
    let config: toml::Table = toml::from_str(contents.as_str())?;
    let project = config.iter().find(|p| p.0 == project_name);

    match project {
        Some((_, values)) => match values.as_table() {
            Some(v) => {
                let mut values_iter = v.iter();
                match values_iter.find(|p| p.0 == "database_url") {
                    Some((_, database_url)) => {
                        let database_token = values_iter
                            .find(|p| p.0 == "database_url")
                            .map(|(_, dt)| dt.to_string());
                        Ok(GeniConfig {
                            database_url: database_url.to_string(),
                            database_token,
                        })
                    }
                    None => bail!("didn't database_url for project {}", project_name),
                }
            }
            None => bail!("didn't find config for {}", project_name),
        },
        None => bail!("didn't find config for {}", project_name),
    }
}
