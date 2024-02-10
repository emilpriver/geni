use geni;

#[tokio::main]
async fn main() {
    geni::migrate(
        "./migrations",       // Path to migrations
        "sqlite://./test.db", // Database URL
        None,                 // Database token if used, only for Turso
        None, // Wait timeout for waiting for the database to be ready, default is 0 seconds
    )
    .await
}
