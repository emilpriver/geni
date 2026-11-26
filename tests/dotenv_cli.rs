use std::{fs, path::Path, process::Command};

fn geni(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_geni"));
    command.current_dir(directory).env_clear();
    command
}

#[test]
fn create_uses_database_url_from_dotenv() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    fs::write(path.join(".env"), "DATABASE_URL=sqlite://./dotenv.sqlite\n").unwrap();

    let output = geni(path).arg("create").output().unwrap();
    assert!(
        output.status.success() && path.join("dotenv.sqlite").is_file(),
        "geni did not create the database: {}",
        String::from_utf8_lossy(&output.stderr),
    );
}

#[test]
fn explicit_database_url_takes_precedence_over_dotenv() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    fs::write(path.join(".env"), "DATABASE_URL=sqlite://./dotenv.sqlite\n").unwrap();

    let environment = geni(path)
        .env("DATABASE_URL", "sqlite://./environment.sqlite")
        .arg("create")
        .output()
        .unwrap();
    assert!(environment.status.success());
    assert!(path.join("environment.sqlite").is_file());
    assert!(!path.join("dotenv.sqlite").exists());

    let explicit = geni(path)
        .env("DATABASE_URL", "sqlite://./environment.sqlite")
        .args(["--database-url", "sqlite://./flag.sqlite", "create"])
        .output()
        .unwrap();
    assert!(explicit.status.success());
    assert!(path.join("flag.sqlite").is_file());
    assert!(!path.join("dotenv.sqlite").exists());
}
