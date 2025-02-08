use anyhow::{bail, Result};
use sqlx::database;
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

fn clean_string(value: String) -> String {
    // Strings in toml starts with " and end with " and we need to remove these
    let mut cleaned = value;
    if let Some(s) = cleaned.strip_prefix('"') {
        cleaned = s.to_string()
    }
    if let Some(s) = cleaned.strip_suffix('"') {
        cleaned = s.to_string()
    }

    cleaned
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
                        let mut database_token = v
                            .iter()
                            .find(|p| p.0 == "database_token")
                            .map(|(_, dt)| dt.to_string());

                        let database_url_as_string = clean_string(database_url.to_string());
                        let cleaned_url = match database_url_as_string.starts_with("env:") {
                            true => {
                                if let Some(without_prefix) =
                                    database_url_as_string.strip_prefix("env:")
                                {
                                    env::var(without_prefix)?
                                } else {
                                    database_url_as_string
                                }
                            }
                            false => database_url_as_string,
                        };

                        if let Some(database_token_exists) = database_token {
                            let database_token_as_string =
                                clean_string(database_token_exists.to_string());
                            let cleaned_token = match database_token_as_string.starts_with("env:") {
                                true => {
                                    if let Some(without_prefix) =
                                        database_token_as_string.strip_prefix("env:")
                                    {
                                        env::var(without_prefix)?
                                    } else {
                                        database_token_as_string
                                    }
                                }
                                false => database_token_as_string,
                            };
                            database_token = Some(cleaned_token)
                        }

                        Ok(GeniConfig {
                            database_url: cleaned_url,
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
