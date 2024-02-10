use anyhow::Result;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub async fn write_to_schema_file(
    content: String,
    migrations_folder: String,
    schema_file: String,
) -> Result<()> {
    let schema_path = format!("{}/{}", migrations_folder, schema_file);
    let path = Path::new(schema_path.as_str());

    if File::open(path.to_str().unwrap()).is_err() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        File::create(&schema_path)?;
    };

    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(schema_path.as_str())?;

    file.write_all(content.as_bytes())?;
    Ok(())
}
