use geni;

#[tokio::main]
async fn main() {
    // Migrate the database
    geni::migrate_database(
        "sqlite://./test.db".to_string(), // Database URL
        None,                             // Database Token
        "migrations".to_string(),         // Migration Table
        "./migrations".to_string(),       // Migration Folder
        "schema.sql".to_string(),         // Schema File
        Some(30),                         // Wait timeout for the database to be ready
        false,                            // Dump Schema
    )
    .await
    .unwrap();

    // Rollbacka changes
    geni::migate_down(
        "sqlite://./test.db".to_string(), // Database URL
        None,                             // Database Token
        "migrations".to_string(),         // Migration Table
        "./migrations".to_string(),       // Migration Folder
        "schema.sql".to_string(),         // Schema File
        Some(30),                         // Wait timeout for the database to be ready
        false,                            // Dump Schema
        1,                                // Rollback Amount
    )
    .await
    .unwrap();

    // Create a database
    geni::create_database(
        "sqlite://./test.db".to_string(), // Database URL
        None,                             // Database Token
        "migrations".to_string(),         // Migration Table
        "./migrations".to_string(),       // Migration Folder
        "schema.sql".to_string(),         // Schema File
        Some(30),                         // Wait timeout for the database to be ready
    )
    .await
    .unwrap();

    // Create a database
    geni::dump_database(
        "sqlite://./test.db".to_string(), // Database URL
        None,                             // Database Token
        "migrations".to_string(),         // Migration Table
        "./migrations".to_string(),       // Migration Folder
        "schema.sql".to_string(),         // Schema File
        Some(30),                         // Wait timeout for the database to be ready
    )
    .await
    .unwrap();

    geni::new_migration(
        "./migration".to_string(), // Migration Folder
        &"test".to_string(),       // New migration name
    );

    ()
}
