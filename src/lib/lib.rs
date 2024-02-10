mod config;
mod database_drivers;
mod dump;
mod generate;
mod integration_test;
mod management;
mod migrate;
mod status;
mod utils;

pub async fn migrate_database(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    dump_schema: bool,
) -> anyhow::Result<()> {
    migrate::up(
        database_url,
        database_token,
        migration_table,
        migration_folder,
        schema_file,
        wait_timeout,
        dump_schema,
    )
    .await
}

pub async fn migate_down(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    dump_schema: bool,
    rollback_amount: i64,
) -> anyhow::Result<()> {
    migrate::down(
        database_url,
        database_token,
        migration_table,
        migration_folder,
        schema_file,
        wait_timeout,
        dump_schema,
        &rollback_amount,
    )
    .await
}

pub async fn create_database(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
) -> anyhow::Result<()> {
    management::create(
        database_url,
        database_token,
        wait_timeout,
        migration_table,
        migration_folder,
        schema_file,
    )
    .await
}

pub async fn drop_database(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
) -> anyhow::Result<()> {
    management::drop(
        database_url,
        database_token,
        wait_timeout,
        migration_table,
        migration_folder,
        schema_file,
    )
    .await
}

pub async fn new_migration(migration_path: String, name: &String) -> anyhow::Result<()> {
    generate::generate_new_migration(&migration_path, name)
}

pub async fn status_migrations(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
    verbose: bool,
) -> anyhow::Result<()> {
    status::status(
        database_url,
        database_token,
        migration_table,
        migration_folder,
        schema_file,
        wait_timeout,
        verbose,
    )
    .await
}

pub async fn dump_database(
    database_url: String,
    database_token: Option<String>,
    migration_table: String,
    migration_folder: String,
    schema_file: String,
    wait_timeout: Option<usize>,
) -> anyhow::Result<()> {
    dump::dump(
        database_url,
        database_token,
        migration_table,
        migration_folder,
        schema_file,
        wait_timeout,
    )
    .await
}
