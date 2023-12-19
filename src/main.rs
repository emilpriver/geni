use clap::{Arg, ArgAction, Command};
use log::{error, info};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

mod config;
mod database_drivers;
mod generate;
mod migrate;

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");

    let matches = Command::new("libsql-migeate")
        .about("Migrate tool for libsql")
        .version("1.0.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Emil Privér")
        .subcommands([
            Command::new("new").about("Create new migration").arg(
                Arg::new("name")
                    .short('n')
                    .long("name")
                    .help("Name for your new migration")
                    .action(ArgAction::Set)
                    .num_args(1),
            ),
            Command::new("up").about("Migrate to the latest version"),
            Command::new("down")
                .about("Rollback to last migration")
                .arg(
                    Arg::new("amount")
                        .short('a')
                        .long("amount")
                        .help("Amount of migrations to rollback")
                        .action(ArgAction::Set)
                        .num_args(1),
                ),
        ])
        .get_matches();

    match matches.subcommand() {
        Some(("new", query_matches)) => {
            let name = query_matches.get_one::<String>("name").unwrap();
            match generate::generate_new_migration(name) {
                Err(err) => {
                    error!("{:?}", err)
                }
                Ok(_) => info!("Success"),
            };
        }
        Some(("up", ..)) => {
            match migrate::up().await {
                Err(err) => {
                    error!("{:?}", err)
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

            match migrate::down(&rollback_amount).await {
                Err(err) => {
                    error!("{:?}", err)
                }
                Ok(_) => info!("Success"),
            };
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
