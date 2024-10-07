use clap::{crate_authors, crate_description, crate_version, Arg, ArgAction, Command};
use log::{error, info};
use simplelog::{ColorChoice, Config, LevelFilter, TermLogger, TerminalMode};

mod config;

#[tokio::main]
async fn main() {
    TermLogger::init(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )
    .expect("Failed to initialize logger");

    let matches = Command::new("geni")
        .about(crate_description!())
        .version(format!("v{}", crate_version!()))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .name("geni")
        .author(crate_authors!())
        .arg(
            Arg::new("project")
                .short('p')
                .long("project")
                .help("The project in geni.toml to use")
                .action(ArgAction::Set)
                .num_args(0..=1),
        )
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

    let migration_path = config::migration_folder();
    let wait_timeout = config::wait_timeout();
    let migrations_table = config::migrations_table();
    let migrations_folder = config::migration_folder();
    let schema_file = config::schema_file();
    let dump_schema = config::dump_schema_file();

    let (database_url, database_token) = match matches.get_one::<String>("project") {
        Some(project) => match config::load_config_file(project) {
            Ok(p) => (p.database_url, p.database_token),
            Err(e) => {
                error!("Failed to load config file: {}", e);
                return;
            }
        },
        None => {
            let token = config::database_token();
            let url = match config::database_url() {
                Ok(url) => url,
                Err(..) => {
                    error!(
                        "No database url found, please set the DATABASE_URL environment variable"
                    );
                    return;
                }
            };

            (url, token)
        }
    };

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
