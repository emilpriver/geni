use log::{error, info};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

mod config;
mod tunnel;

async fn resolve_database_connection_or_exit(
    matches: &clap::ArgMatches,
) -> Option<config::ResolvedDatabaseConnection> {
    match config::resolve_database_connection(matches).await {
        Ok(connection) => Some(connection),
        Err(err) => {
            error!("{:?}", err);
            None
        }
    }
}

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");

    let matches = config::cli_command().get_matches();

    let migration_path = config::migration_folder();
    let wait_timeout = config::wait_timeout();
    let migrations_table = config::migrations_table();
    let migrations_folder = config::migration_folder();
    let schema_file = config::schema_file();
    let dump_schema = config::dump_schema_file();

    match matches.subcommand() {
        Some(("new", query_matches)) => {
            let name = query_matches.get_one::<String>("name").unwrap();
            match geni::new_migration(migration_path, name).await {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("create", ..)) => {
            let Some(database_connection) = resolve_database_connection_or_exit(&matches).await
            else {
                return;
            };
            let database_url = database_connection.database_url.clone();
            let database_token = database_connection.database_token.clone();

            match geni::create_database(
                database_url,
                database_token,
                migrations_table,
                migrations_folder,
                schema_file,
                Some(wait_timeout),
            )
            .await
            {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("drop", ..)) => {
            let Some(database_connection) = resolve_database_connection_or_exit(&matches).await
            else {
                return;
            };
            let database_url = database_connection.database_url.clone();
            let database_token = database_connection.database_token.clone();

            match geni::drop_database(
                database_url,
                database_token,
                migrations_table,
                migrations_folder,
                schema_file,
                Some(wait_timeout),
            )
            .await
            {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("up", ..)) => {
            let Some(database_connection) = resolve_database_connection_or_exit(&matches).await
            else {
                return;
            };
            let database_url = database_connection.database_url.clone();
            let database_token = database_connection.database_token.clone();

            match geni::migrate_database(
                database_url,
                database_token,
                migrations_table,
                migrations_folder,
                schema_file,
                Some(wait_timeout),
                dump_schema,
            )
            .await
            {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("down", query_matches)) => {
            let Some(database_connection) = resolve_database_connection_or_exit(&matches).await
            else {
                return;
            };
            let database_url = database_connection.database_url.clone();
            let database_token = database_connection.database_token.clone();
            let rollback_amount = query_matches
                .get_one::<String>("amount")
                .unwrap_or(&"1".to_string())
                .parse::<i64>()
                .expect("Couldn't parse amount, is it a number?");

            match geni::migate_down(
                database_url,
                database_token,
                migrations_table,
                migrations_folder,
                schema_file,
                Some(wait_timeout),
                dump_schema,
                rollback_amount,
            )
            .await
            {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("status", query_matches)) => {
            let Some(database_connection) = resolve_database_connection_or_exit(&matches).await
            else {
                return;
            };
            let database_url = database_connection.database_url.clone();
            let database_token = database_connection.database_token.clone();

            let verbose = query_matches.contains_id("verbose");

            if let Err(err) = geni::status_migrations(
                database_url,
                database_token,
                migrations_table,
                migrations_folder,
                schema_file,
                Some(wait_timeout),
                verbose,
            )
            .await
            {
                error!("{:?}", err);
                std::process::exit(1);
            }
        }
        Some(("dump", ..)) => {
            let Some(database_connection) = resolve_database_connection_or_exit(&matches).await
            else {
                return;
            };
            let database_url = database_connection.database_url.clone();
            let database_token = database_connection.database_token.clone();

            match geni::dump_database(
                database_url,
                database_token,
                migrations_table,
                migrations_folder,
                schema_file,
                Some(wait_timeout),
            )
            .await
            {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
