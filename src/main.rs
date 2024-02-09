use clap::{crate_authors, crate_description, crate_version, Arg, ArgAction, Command};
use log::{error, info};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

mod config;
mod database_drivers;
mod dump;
mod generate;
mod integration_test;
mod management;
mod migrate;
mod status;
mod utils;

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");

    let migration_path = config::migration_folder();
    let database_url = match config::database_url() {
        Ok(url) => url,
        Err(..) => {
            error!("No database url found, please set the DATABASE_URL environment variable");
            return;
        }
    };
    let wait_timeout = config::wait_timeout();

    let database_token = config::database_token();

    let matches = Command::new("geni")
        .about(crate_description!())
        .version(format!("v{}", crate_version!()))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .name("geni")
        .author(crate_authors!())
        .subcommands([
            Command::new("new")
                .about("Create new migration")
                .arg(Arg::new("name").required(true).index(1)),
            Command::new("up").about("Migrate to the latest version"),
            Command::new("down")
                .about("Rollback to last migration")
                .arg(
                    Arg::new("amount")
                        .short('a')
                        .long("amount")
                        .help("Amount of migrations to rollback")
                        .action(ArgAction::Set)
                        .num_args(0..=1),
                ),
            Command::new("create").about("Create database"),
            Command::new("drop").about("Drop database"),
            Command::new("status")
                .about("Show current migrations to apply")
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .help("Include migration content for the non applied migrations")
                        .action(ArgAction::Set)
                        .num_args(0..=1),
                ),
            Command::new("dump").about("Dump database structure"),
        ])
        .get_matches();

    match matches.subcommand() {
        Some(("new", query_matches)) => {
            let name = query_matches.get_one::<String>("name").unwrap();
            match generate::generate_new_migration(&migration_path, name) {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("create", ..)) => {
            match management::create(&database_url, database_token, Some(wait_timeout)).await {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("drop", ..)) => {
            match management::drop(&database_url, database_token, Some(wait_timeout)).await {
                Err(err) => {
                    error!("{:?}", err);
                    std::process::exit(1);
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("up", ..)) => {
            match migrate::up(
                &migration_path,
                &database_url,
                database_token,
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
        Some(("down", query_matches)) => {
            let rollback_amount = query_matches
                .get_one::<String>("amount")
                .unwrap_or(&"1".to_string())
                .parse::<i64>()
                .expect("Couldn't parse amount, is it a number?");

            match migrate::down(
                &migration_path,
                &database_url,
                database_token,
                Some(wait_timeout),
                &rollback_amount,
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
            let verbose = query_matches.contains_id("verbose");

            if let Err(err) = status::status(
                &migration_path,
                &database_url,
                database_token,
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
            match dump::dump(&database_url, database_token, Some(wait_timeout)).await {
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

// Revert X amount of migrations
pub async fn migrate_up(
    migration_path: String,
    database_url: String,
    database_token: Option<String>,
    wait_timeout: Option<usize>,
) -> anyhow::Result<()> {
    migrate::up(&migration_path, &database_url, database_token, wait_timeout).await
}

// Revert X amount of migrations
pub async fn migrate_down(
    migration_path: String,
    database_url: String,
    database_token: Option<String>,
    wait_timeout: Option<usize>,
    down_migration_amount: i64,
) -> anyhow::Result<()> {
    migrate::down(
        &migration_path,
        &database_url,
        database_token,
        wait_timeout,
        &down_migration_amount,
    )
    .await
}

// Create the database from given database_url
pub async fn create_database(
    database_url: String,
    database_token: Option<String>,
    wait_timeout: Option<usize>,
) -> anyhow::Result<()> {
    management::create(&database_url, database_token, wait_timeout).await
}
