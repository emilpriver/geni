use crate::tunnel::{self, SshTunnelConfig, SshTunnelGuard};
use anyhow::{bail, Result};
use clap::{
    crate_authors, crate_description, crate_version, value_parser, Arg, ArgAction, ArgMatches,
    Command,
};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseOptions {
    pub database_url: String,
    pub database_token: Option<String>,
    pub wait_timeout: usize,
    pub tunnel: Option<SshTunnelConfig>,
}

pub struct ResolvedDatabaseConnection {
    pub database_url: String,
    pub database_token: Option<String>,
    _tunnel: Option<SshTunnelGuard>,
}

impl DatabaseOptions {
    pub async fn resolve(self) -> Result<ResolvedDatabaseConnection> {
        match self.tunnel {
            Some(tunnel_config) => {
                let tunnel = tunnel::establish_tunnel(
                    self.database_url.clone(),
                    tunnel_config,
                    self.wait_timeout,
                )
                .await?;

                Ok(ResolvedDatabaseConnection {
                    database_url: tunnel.database_url,
                    database_token: self.database_token,
                    _tunnel: Some(tunnel.guard),
                })
            }
            None => Ok(ResolvedDatabaseConnection {
                database_url: self.database_url,
                database_token: self.database_token,
                _tunnel: None,
            }),
        }
    }
}

pub fn cli_command() -> Command {
    Command::new("geni")
        .about(crate_description!())
        .version(format!("v{}", crate_version!()))
        .subcommand_required(true)
        .arg_required_else_help(true)
        .name("geni")
        .author(crate_authors!())
        .args(database_args())
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
}

pub fn database_options(matches: &ArgMatches) -> Result<DatabaseOptions> {
    let database_url = matches
        .get_one::<String>("database-url")
        .cloned()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("missing DATABASE_URL env variable or --database-url flag")
        })?;

    let ssh_host = matches.get_one::<String>("ssh-host").cloned();
    let ssh_user = matches.get_one::<String>("ssh-user").cloned();
    let ssh_port = matches.get_one::<u16>("ssh-port").copied();
    let ssh_identity_file = matches.get_one::<PathBuf>("ssh-identity-file").cloned();
    let ssh_local_port = matches.get_one::<u16>("ssh-local-port").copied();
    let ssh_remote_host = matches.get_one::<String>("ssh-remote-host").cloned();
    let ssh_remote_port = matches.get_one::<u16>("ssh-remote-port").copied();

    let has_ssh_settings = ssh_user.is_some()
        || ssh_port.is_some()
        || ssh_identity_file.is_some()
        || ssh_local_port.is_some()
        || ssh_remote_host.is_some()
        || ssh_remote_port.is_some();

    if ssh_host.is_none() && has_ssh_settings {
        bail!("SSH tunnel settings were provided without --ssh-host or DATABASE_SSH_HOST");
    }

    let tunnel = ssh_host.map(|ssh_host| SshTunnelConfig {
        ssh_host,
        ssh_user,
        ssh_port,
        ssh_identity_file,
        local_port: ssh_local_port,
        remote_host: ssh_remote_host,
        remote_port: ssh_remote_port,
    });

    Ok(DatabaseOptions {
        database_url,
        database_token: database_token(),
        wait_timeout: wait_timeout(),
        tunnel,
    })
}

pub async fn resolve_database_connection(
    matches: &ArgMatches,
) -> Result<ResolvedDatabaseConnection> {
    database_options(matches)?.resolve().await
}

fn database_args() -> [Arg; 8] {
    [
        Arg::new("database-url")
            .long("database-url")
            .global(true)
            .env("DATABASE_URL")
            .help("Database URL used for commands that connect to a database"),
        Arg::new("ssh-host")
            .long("ssh-host")
            .global(true)
            .env("DATABASE_SSH_HOST")
            .help("SSH host or ~/.ssh/config alias used to open a tunnel"),
        Arg::new("ssh-user")
            .long("ssh-user")
            .global(true)
            .env("DATABASE_SSH_USER")
            .help("SSH username override for the tunnel"),
        Arg::new("ssh-port")
            .long("ssh-port")
            .global(true)
            .env("DATABASE_SSH_PORT")
            .value_parser(value_parser!(u16))
            .help("SSH port override for the tunnel"),
        Arg::new("ssh-identity-file")
            .long("ssh-identity-file")
            .global(true)
            .env("DATABASE_SSH_IDENTITY_FILE")
            .value_parser(value_parser!(PathBuf))
            .help("SSH identity file used when opening the tunnel"),
        Arg::new("ssh-local-port")
            .long("ssh-local-port")
            .global(true)
            .env("DATABASE_SSH_LOCAL_PORT")
            .value_parser(value_parser!(u16))
            .help("Local port to bind for the SSH tunnel"),
        Arg::new("ssh-remote-host")
            .long("ssh-remote-host")
            .global(true)
            .env("DATABASE_SSH_REMOTE_HOST")
            .help("Remote host reachable from the SSH server; defaults to the DATABASE_URL host"),
        Arg::new("ssh-remote-port")
            .long("ssh-remote-port")
            .global(true)
            .env("DATABASE_SSH_REMOTE_PORT")
            .value_parser(value_parser!(u16))
            .help("Remote port reachable from the SSH server; defaults to the DATABASE_URL port"),
    ]
}

pub fn migration_folder() -> String {
    if let Ok(v) = env::var("DATABASE_MIGRATIONS_FOLDER") {
        if !v.is_empty() {
            return v;
        }
    }

    "./migrations".to_string()
}

pub fn database_token() -> Option<String> {
    if let Ok(v) = env::var("DATABASE_TOKEN") {
        if !v.is_empty() {
            return Some(v);
        }
    }

    None
}

pub fn wait_timeout() -> usize {
    if let Ok(v) = env::var("DATABASE_WAIT_TIMEOUT") {
        if !v.is_empty() {
            if let Ok(v) = v.parse::<usize>() {
                return v;
            }
        }
    }

    30
}

pub fn dump_schema_file() -> bool {
    if let Ok(v) = env::var("DATABASE_NO_DUMP_SCHEMA") {
        if v == "true" {
            return false;
        }
    }

    true
}

pub fn schema_file() -> String {
    if let Ok(v) = env::var("DATABASE_SCHEMA_FILE") {
        if !v.is_empty() {
            return v;
        }
    }

    "schema.sql".to_string()
}

pub fn migrations_table() -> String {
    if let Ok(v) = env::var("DATABASE_MIGRATIONS_TABLE") {
        if !v.is_empty() {
            return v;
        }
    }

    "schema_migrations".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::ffi::OsString;
    use std::path::PathBuf;

    struct EnvGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = env::var_os(key);
            env::set_var(key, value);
            Self { key, previous }
        }

        fn unset(key: &'static str) -> Self {
            let previous = env::var_os(key);
            env::remove_var(key);
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => env::set_var(self.key, value),
                None => env::remove_var(self.key),
            }
        }
    }

    #[test]
    fn test_new_subcommand_does_not_require_database_url() {
        let matches = cli_command()
            .try_get_matches_from(["geni", "new", "create_users"])
            .unwrap();

        assert_eq!(matches.subcommand_name(), Some("new"));
    }

    #[test]
    #[serial]
    fn test_database_url_cli_overrides_env() {
        let _database_url = EnvGuard::set(
            "DATABASE_URL",
            "postgres://env:env@env.internal:5432/env_db?sslmode=disable",
        );

        let matches = cli_command()
            .try_get_matches_from([
                "geni",
                "--database-url",
                "postgres://cli:cli@cli.internal:5432/cli_db?sslmode=disable",
                "up",
            ])
            .unwrap();

        let options = database_options(&matches).unwrap();
        assert_eq!(
            options.database_url,
            "postgres://cli:cli@cli.internal:5432/cli_db?sslmode=disable"
        );
    }

    #[test]
    #[serial]
    fn test_database_url_env_fallback() {
        let _database_url = EnvGuard::set(
            "DATABASE_URL",
            "postgres://env:env@env.internal:5432/env_db?sslmode=disable",
        );

        let matches = cli_command().try_get_matches_from(["geni", "up"]).unwrap();
        let options = database_options(&matches).unwrap();

        assert_eq!(
            options.database_url,
            "postgres://env:env@env.internal:5432/env_db?sslmode=disable"
        );
    }

    #[test]
    #[serial]
    fn test_database_options_requires_database_url() {
        let _database_url = EnvGuard::unset("DATABASE_URL");

        let matches = cli_command().try_get_matches_from(["geni", "up"]).unwrap();
        let error = database_options(&matches).unwrap_err();

        assert_eq!(
            error.to_string(),
            "missing DATABASE_URL env variable or --database-url flag"
        );
    }

    #[test]
    #[serial]
    fn test_database_options_requires_ssh_host_for_partial_ssh_config() {
        let _database_url = EnvGuard::set(
            "DATABASE_URL",
            "postgres://env:env@env.internal:5432/env_db?sslmode=disable",
        );

        let matches = cli_command()
            .try_get_matches_from(["geni", "--ssh-user", "deploy", "up"])
            .unwrap();

        let error = database_options(&matches).unwrap_err();
        assert_eq!(
            error.to_string(),
            "SSH tunnel settings were provided without --ssh-host or DATABASE_SSH_HOST"
        );
    }

    #[test]
    #[serial]
    fn test_database_options_parses_tunnel_from_args() {
        let _database_url = EnvGuard::unset("DATABASE_URL");
        let matches = cli_command()
            .try_get_matches_from([
                "geni",
                "--database-url",
                "postgres://user:password@db.internal:5432/app?sslmode=disable",
                "--ssh-host",
                "bastion",
                "--ssh-user",
                "deploy",
                "--ssh-port",
                "2202",
                "--ssh-identity-file",
                "/tmp/id_ed25519",
                "--ssh-local-port",
                "6432",
                "--ssh-remote-host",
                "postgres.internal",
                "--ssh-remote-port",
                "5433",
                "up",
            ])
            .unwrap();

        let options = database_options(&matches).unwrap();

        assert_eq!(
            options.tunnel,
            Some(SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: Some("deploy".to_string()),
                ssh_port: Some(2202),
                ssh_identity_file: Some(PathBuf::from("/tmp/id_ed25519")),
                local_port: Some(6432),
                remote_host: Some("postgres.internal".to_string()),
                remote_port: Some(5433),
            })
        );
    }

    #[test]
    #[serial]
    fn test_ssh_host_cli_overrides_env() {
        let _database_url = EnvGuard::set(
            "DATABASE_URL",
            "postgres://env:env@env.internal:5432/env_db?sslmode=disable",
        );
        let _ssh_host = EnvGuard::set("DATABASE_SSH_HOST", "env-bastion");

        let matches = cli_command()
            .try_get_matches_from(["geni", "--ssh-host", "cli-bastion", "up"])
            .unwrap();

        let options = database_options(&matches).unwrap();
        assert_eq!(options.tunnel.unwrap().ssh_host, "cli-bastion");
    }
}
