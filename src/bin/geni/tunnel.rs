use anyhow::{bail, Context, Result};
use log::info;
use std::ffi::OsString;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::time::{sleep, Instant};
use url::Url;
use which::which;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTunnelConfig {
    pub ssh_host: String,
    pub ssh_user: Option<String>,
    pub ssh_port: Option<u16>,
    pub ssh_identity_file: Option<PathBuf>,
    pub local_port: Option<u16>,
    pub remote_host: Option<String>,
    pub remote_port: Option<u16>,
}

pub struct EstablishedTunnel {
    pub database_url: String,
    pub guard: SshTunnelGuard,
}

pub struct SshTunnelGuard {
    child: Option<Child>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TunnelPlan {
    ssh_destination: String,
    ssh_args: Vec<OsString>,
    local_port: u16,
    remote_host: String,
    remote_port: u16,
    rewritten_database_url: String,
}

impl SshTunnelGuard {
    fn new(child: Child) -> Self {
        Self { child: Some(child) }
    }

    #[cfg(test)]
    async fn shutdown(&mut self) -> Result<()> {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }

        Ok(())
    }
}

impl Drop for SshTunnelGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.start_kill();

            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    let _ = child.wait().await;
                });
            }
        }
    }
}

pub async fn establish_tunnel(
    database_url: String,
    config: SshTunnelConfig,
    wait_timeout: usize,
) -> Result<EstablishedTunnel> {
    let plan = build_tunnel_plan(&database_url, config)?;
    let ssh_binary = find_ssh_binary()?;

    info!(
        "Opening SSH tunnel via {} to {}:{}",
        plan.ssh_destination, plan.remote_host, plan.remote_port
    );

    let mut child = spawn_ssh_tunnel(&ssh_binary, &plan)?;

    if let Err(error) = wait_for_tunnel_ready(&mut child, plan.local_port, wait_timeout).await {
        let _ = child.start_kill();
        let _ = child.wait().await;
        return Err(error);
    }

    info!(
        "Using local forwarded database address {}",
        redact_database_url(&plan.rewritten_database_url)
    );

    Ok(EstablishedTunnel {
        database_url: plan.rewritten_database_url,
        guard: SshTunnelGuard::new(child),
    })
}

fn build_tunnel_plan(database_url: &str, config: SshTunnelConfig) -> Result<TunnelPlan> {
    let mut parsed_database_url = Url::parse(database_url)
        .with_context(|| format!("invalid database URL: {}", database_url))?;
    let scheme = parsed_database_url.scheme().to_string();

    ensure_supported_tunnel_scheme(&scheme)?;

    let remote_host = config
        .remote_host
        .or_else(|| parsed_database_url.host_str().map(str::to_string))
        .ok_or_else(|| anyhow::anyhow!("database URL must include a host or --ssh-remote-host"))?;

    let remote_port = match config.remote_port {
        Some(port) => port,
        None => parsed_database_url
            .port()
            .or_else(|| default_port_for_scheme(&scheme))
            .ok_or_else(|| {
                anyhow::anyhow!("database URL must include a port or use a supported default port")
            })?,
    };

    let local_port = select_local_port(config.local_port)?;

    parsed_database_url
        .set_host(Some("127.0.0.1"))
        .context("failed to rewrite database URL host for SSH tunnel")?;
    parsed_database_url
        .set_port(Some(local_port))
        .map_err(|()| anyhow::anyhow!("failed to rewrite database URL port for SSH tunnel"))?;

    let mut ssh_args = vec![
        OsString::from("-o"),
        OsString::from("ExitOnForwardFailure=yes"),
        OsString::from("-N"),
    ];

    if let Some(user) = config.ssh_user {
        ssh_args.push(OsString::from("-l"));
        ssh_args.push(OsString::from(user));
    }

    if let Some(port) = config.ssh_port {
        ssh_args.push(OsString::from("-p"));
        ssh_args.push(OsString::from(port.to_string()));
    }

    if let Some(identity_file) = config.ssh_identity_file {
        ssh_args.push(OsString::from("-i"));
        ssh_args.push(identity_file.into_os_string());
    }

    ssh_args.push(OsString::from("-L"));
    ssh_args.push(OsString::from(format!(
        "127.0.0.1:{}:{}:{}",
        local_port, remote_host, remote_port
    )));

    Ok(TunnelPlan {
        ssh_destination: config.ssh_host,
        ssh_args,
        local_port,
        remote_host,
        remote_port,
        rewritten_database_url: parsed_database_url.to_string(),
    })
}

fn ensure_supported_tunnel_scheme(scheme: &str) -> Result<()> {
    match scheme {
        "postgres" | "postgresql" | "psql" | "mysql" | "mariadb" => Ok(()),
        "sqlite" | "sqlite3" | "http" | "https" | "libsql" | "turso" => {
            bail!("SSH tunnels are only supported for Postgres, MySQL, and MariaDB URLs")
        }
        _ => bail!("Unsupported database driver: {}", scheme),
    }
}

fn default_port_for_scheme(scheme: &str) -> Option<u16> {
    match scheme {
        "postgres" | "postgresql" | "psql" => Some(5432),
        "mysql" | "mariadb" => Some(3306),
        _ => None,
    }
}

fn select_local_port(requested_port: Option<u16>) -> Result<u16> {
    if let Some(port) = requested_port {
        return Ok(port);
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .context("failed to select a free local port for the SSH tunnel")?;
    let port = listener
        .local_addr()
        .context("failed to determine the SSH tunnel local port")?
        .port();

    Ok(port)
}

fn find_ssh_binary() -> Result<PathBuf> {
    which("ssh").context("could not find `ssh` in PATH; install OpenSSH and try again")
}

fn spawn_ssh_tunnel(ssh_binary: &Path, plan: &TunnelPlan) -> Result<Child> {
    let mut command = Command::new(ssh_binary);
    command
        .args(plan.ssh_args.iter().map(OsString::as_os_str))
        .arg(&plan.ssh_destination)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command
        .spawn()
        .with_context(|| format!("failed to start SSH tunnel using {}", ssh_binary.display()))
}

async fn wait_for_tunnel_ready(
    child: &mut Child,
    local_port: u16,
    wait_timeout: usize,
) -> Result<()> {
    let timeout = Duration::from_secs(wait_timeout as u64);
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(status) = child
            .try_wait()
            .context("failed to check SSH tunnel process state")?
        {
            bail!(
                "SSH tunnel exited before it became ready with status {}",
                status
            );
        }

        if TcpStream::connect(("127.0.0.1", local_port)).await.is_ok() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            bail!(
                "SSH tunnel did not become ready within {} seconds",
                wait_timeout
            );
        }

        sleep(Duration::from_millis(100)).await;
    }
}

pub fn redact_database_url(database_url: &str) -> String {
    let Ok(mut parsed) = Url::parse(database_url) else {
        return database_url.to_string();
    };

    if parsed.password().is_some() {
        let _ = parsed.set_password(Some("******"));
    }

    parsed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;
    use std::ffi::{OsStr, OsString};
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    #[cfg(unix)]
    use std::process::Command as StdCommand;
    use tempfile::tempdir;

    struct EnvGuard {
        key: &'static str,
        previous: Option<OsString>,
    }

    impl EnvGuard {
        fn set_path(key: &'static str, value: PathBuf) -> Self {
            let previous = env::var_os(key);
            env::set_var(key, value);
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
    fn test_build_tunnel_plan_defaults_remote_target_from_database_url() {
        let plan = build_tunnel_plan(
            "postgres://user:secret@db.internal:5432/app?sslmode=disable",
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: None,
                ssh_port: None,
                ssh_identity_file: None,
                local_port: Some(6432),
                remote_host: None,
                remote_port: None,
            },
        )
        .unwrap();

        assert_eq!(plan.local_port, 6432);
        assert_eq!(plan.remote_host, "db.internal");
        assert_eq!(plan.remote_port, 5432);
        assert_eq!(
            plan.rewritten_database_url,
            "postgres://user:secret@127.0.0.1:6432/app?sslmode=disable"
        );
    }

    #[test]
    fn test_build_tunnel_plan_preserves_query_and_credentials() {
        let plan = build_tunnel_plan(
            "mysql://user:secret@db.internal/app?ssl-mode=DISABLED",
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: Some("deploy".to_string()),
                ssh_port: Some(2202),
                ssh_identity_file: Some(PathBuf::from("/tmp/id_ed25519")),
                local_port: Some(3307),
                remote_host: Some("mysql.internal".to_string()),
                remote_port: Some(3308),
            },
        )
        .unwrap();

        assert_eq!(
            plan.rewritten_database_url,
            "mysql://user:secret@127.0.0.1:3307/app?ssl-mode=DISABLED"
        );
        assert_eq!(
            plan.ssh_args,
            vec![
                OsString::from("-o"),
                OsString::from("ExitOnForwardFailure=yes"),
                OsString::from("-N"),
                OsString::from("-l"),
                OsString::from("deploy"),
                OsString::from("-p"),
                OsString::from("2202"),
                OsString::from("-i"),
                OsString::from("/tmp/id_ed25519"),
                OsString::from("-L"),
                OsString::from("127.0.0.1:3307:mysql.internal:3308"),
            ]
        );
    }

    #[test]
    fn test_build_tunnel_plan_rejects_unsupported_schemes() {
        let error = build_tunnel_plan(
            "sqlite://./database.sqlite",
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: None,
                ssh_port: None,
                ssh_identity_file: None,
                local_port: None,
                remote_host: None,
                remote_port: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "SSH tunnels are only supported for Postgres, MySQL, and MariaDB URLs"
        );
    }

    #[test]
    fn test_build_tunnel_plan_requires_remote_host_when_database_url_has_no_host() {
        let error = build_tunnel_plan(
            "postgres:///app",
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: None,
                ssh_port: None,
                ssh_identity_file: None,
                local_port: Some(5439),
                remote_host: None,
                remote_port: None,
            },
        )
        .unwrap_err();

        assert_eq!(
            error.to_string(),
            "database URL must include a host or --ssh-remote-host"
        );
    }

    #[test]
    fn test_redact_database_url_hides_passwords() {
        assert_eq!(
            redact_database_url("postgres://user:secret@127.0.0.1:5432/app?sslmode=disable"),
            "postgres://user:******@127.0.0.1:5432/app?sslmode=disable"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial]
    async fn test_establish_tunnel_uses_stub_ssh_binary() -> Result<()> {
        if which("python3").is_err() {
            return Ok(());
        }

        let temp_dir = tempdir()?;
        let ssh_path = temp_dir.path().join("ssh");
        let args_path = temp_dir.path().join("ssh-args.txt");

        let script = format!(
            "#!/bin/sh\nprintf '%s\n' \"$@\" > \"{}\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-L\" ]; then\n    local_forward=\"$2\"\n    break\n  fi\n  shift\n done\nlocal_port=$(printf '%s' \"$local_forward\" | cut -d: -f2)\npython3 - <<'PY' \"$local_port\"\nimport socket, sys\nport = int(sys.argv[1])\nserver = socket.socket()\nserver.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\nserver.bind(('127.0.0.1', port))\nserver.listen(1)\nwhile True:\n    conn, _ = server.accept()\n    conn.close()\nPY\n",
            args_path.display()
        );

        fs::write(&ssh_path, script)?;
        let mut permissions = fs::metadata(&ssh_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ssh_path, permissions)?;

        let existing_path = env::var_os("PATH").unwrap_or_default();
        let mut combined_path = OsString::from(temp_dir.path().as_os_str());
        combined_path.push(OsStr::new(":"));
        combined_path.push(existing_path);
        let _path_guard = EnvGuard::set_path("PATH", PathBuf::from(combined_path));

        let mut tunnel = establish_tunnel(
            "postgres://user:secret@db.internal:5432/app?sslmode=disable".to_string(),
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: Some("deploy".to_string()),
                ssh_port: Some(2202),
                ssh_identity_file: Some(PathBuf::from("/tmp/id_ed25519")),
                local_port: Some(6432),
                remote_host: Some("postgres.internal".to_string()),
                remote_port: Some(5433),
            },
            2,
        )
        .await?;

        assert_eq!(
            tunnel.database_url,
            "postgres://user:secret@127.0.0.1:6432/app?sslmode=disable"
        );

        let args = fs::read_to_string(&args_path)?;
        assert!(args.contains("-o"));
        assert!(args.contains("ExitOnForwardFailure=yes"));
        assert!(args.contains("-N"));
        assert!(args.contains("-l"));
        assert!(args.contains("deploy"));
        assert!(args.contains("-p"));
        assert!(args.contains("2202"));
        assert!(args.contains("-i"));
        assert!(args.contains("/tmp/id_ed25519"));
        assert!(args.contains("127.0.0.1:6432:postgres.internal:5433"));
        assert!(args.contains("bastion"));

        tunnel.guard.shutdown().await?;

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial]
    async fn test_establish_tunnel_errors_when_ssh_exits_early() -> Result<()> {
        let temp_dir = tempdir()?;
        let ssh_path = temp_dir.path().join("ssh");

        fs::write(&ssh_path, "#!/bin/sh\nexit 12\n")?;
        let mut permissions = fs::metadata(&ssh_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ssh_path, permissions)?;

        let existing_path = env::var_os("PATH").unwrap_or_default();
        let mut combined_path = OsString::from(temp_dir.path().as_os_str());
        combined_path.push(OsStr::new(":"));
        combined_path.push(existing_path);
        let _path_guard = EnvGuard::set_path("PATH", PathBuf::from(combined_path));

        let error = establish_tunnel(
            "postgres://user:secret@db.internal:5432/app?sslmode=disable".to_string(),
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: None,
                ssh_port: None,
                ssh_identity_file: None,
                local_port: Some(6432),
                remote_host: None,
                remote_port: None,
            },
            1,
        )
        .await
        .err()
        .expect("expected SSH tunnel startup to fail");

        assert!(error
            .to_string()
            .contains("SSH tunnel exited before it became ready"));

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial]
    async fn test_tunnel_guard_shutdown_terminates_child() -> Result<()> {
        let temp_dir = tempdir()?;
        let ssh_path = temp_dir.path().join("ssh");
        let pid_path = temp_dir.path().join("ssh.pid");
        if which("python3").is_err() {
            return Ok(());
        }

        let script = format!(
            "#!/bin/sh\necho $$ > \"{}\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-L\" ]; then\n    local_forward=\"$2\"\n    break\n  fi\n  shift\n done\nlocal_port=$(printf '%s' \"$local_forward\" | cut -d: -f2)\npython3 - <<'PY' \"$local_port\"\nimport socket, sys\nport = int(sys.argv[1])\nserver = socket.socket()\nserver.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\nserver.bind(('127.0.0.1', port))\nserver.listen(1)\nwhile True:\n    conn, _ = server.accept()\n    conn.close()\nPY\n",
            pid_path.display()
        );
        fs::write(&ssh_path, script)?;
        let mut permissions = fs::metadata(&ssh_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ssh_path, permissions)?;

        let existing_path = env::var_os("PATH").unwrap_or_default();
        let mut combined_path = OsString::from(temp_dir.path().as_os_str());
        combined_path.push(OsStr::new(":"));
        combined_path.push(existing_path);
        let _path_guard = EnvGuard::set_path("PATH", PathBuf::from(combined_path));

        let mut tunnel = establish_tunnel(
            "postgres://user:secret@db.internal:5432/app?sslmode=disable".to_string(),
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: None,
                ssh_port: None,
                ssh_identity_file: None,
                local_port: Some(6433),
                remote_host: Some("127.0.0.1".to_string()),
                remote_port: Some(5432),
            },
            1,
        )
        .await?;

        let mut pid = None;
        for _ in 0..20 {
            if let Ok(contents) = fs::read_to_string(&pid_path) {
                pid = Some(contents.trim().to_string());
                break;
            }

            sleep(Duration::from_millis(50)).await;
        }

        let pid = pid.context("stub SSH process did not publish its PID")?;

        tunnel.guard.shutdown().await?;
        sleep(Duration::from_millis(100)).await;

        let status = StdCommand::new("sh")
            .arg("-c")
            .arg(format!("kill -0 {} >/dev/null 2>&1", pid))
            .status()?;

        assert!(!status.success());

        Ok(())
    }
}
