use clap::{Arg, ArgAction, Command};

use crate::generate::generate_new_migration;

mod generate;

fn main() {
    let matches = Command::new("libsql-migeate")
        .about("Migrate tool for libsql")
        .version("1.0.0")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .author("Emil Privér")
        .subcommand(
            Command::new("new")
                .short_flag('n')
                .long_flag("new")
                .about("Create new migration")
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .help("Name for your new migration")
                        .action(ArgAction::Set)
                        .num_args(1),
                ),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("new", query_matches)) => {
            let name = query_matches.get_one::<String>("name").unwrap();
            let new = generate_new_migration(name);
            println!("{:?}", &new);
        }
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachable
    }
}
