use std::env;

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("MIGRATION_DIR") {
        return v;
    }

    "./migrations".to_string()
}
