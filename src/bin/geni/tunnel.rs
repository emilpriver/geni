use anyhow::{bail, Context, Result};
use log::info;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::process::{Child, Command};
use tokio::time::{sleep, Instant};
use url::Url;
use which::which;

const AUTO_LOCAL_PORT_START: u16 = 60000;
const AUTO_LOCAL_PORT_END: u16 = 65000;
const AUTO_LOCAL_PORT_COUNT: u16 = AUTO_LOCAL_PORT_END - AUTO_LOCAL_PORT_START + 1;

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
    requested_local_port: Option<u16>,
    remote_host: String,
    remote_port: u16,
    database_url: Url,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SshLogState {
    None,
    Listening,
    PortInUse,
}

enum TunnelAttemptError {
    PortInUse,
    Fatal(anyhow::Error),
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
    let deadline = Instant::now() + Duration::from_secs(wait_timeout as u64);
    let candidate_ports = local_port_candidates(plan.requested_local_port, auto_local_port_seed()?);

    info!(
        "Opening SSH tunnel via {} to {}:{}",
        plan.ssh_destination, plan.remote_host, plan.remote_port
    );

    for local_port in candidate_ports {
        if plan.requested_local_port.is_none() && Instant::now() >= deadline {
            break;
        }

        match try_start_ssh_tunnel(&ssh_binary, &plan, local_port, deadline, wait_timeout).await {
            Ok(child) => {
                let rewritten_database_url = rewrite_database_url(&plan.database_url, local_port)?;

                info!(
                    "Using local forwarded database address {}",
                    redact_database_url(&rewritten_database_url)
                );

                return Ok(EstablishedTunnel {
                    database_url: rewritten_database_url,
                    guard: SshTunnelGuard::new(child),
                });
            }
            Err(TunnelAttemptError::PortInUse) => {
                if plan.requested_local_port.is_some() {
                    bail!(
                        "requested SSH tunnel local port {} is already in use",
                        local_port
                    );
                }

                if Instant::now() >= deadline {
                    break;
                }
            }
            Err(TunnelAttemptError::Fatal(error)) => return Err(error),
        }
    }

    bail!(
        "failed to allocate a local SSH tunnel port in {}..={}; specify --ssh-local-port to choose one explicitly",
        AUTO_LOCAL_PORT_START,
        AUTO_LOCAL_PORT_END
    )
}

fn build_tunnel_plan(database_url: &str, config: SshTunnelConfig) -> Result<TunnelPlan> {
    let parsed_database_url = Url::parse(database_url).with_context(|| {
        format!(
            "invalid database URL: {}",
            redact_database_url(database_url)
        )
    })?;
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

    Ok(TunnelPlan {
        ssh_destination: config.ssh_host,
        ssh_args,
        requested_local_port: config.local_port,
        remote_host,
        remote_port,
        database_url: parsed_database_url,
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

fn local_port_candidates(requested_port: Option<u16>, seed: u16) -> Vec<u16> {
    if let Some(port) = requested_port {
        return vec![port];
    }

    let start_offset = u32::from(seed % AUTO_LOCAL_PORT_COUNT);
    let range_len = u32::from(AUTO_LOCAL_PORT_COUNT);

    (0..range_len)
        .map(|offset| {
            let candidate_offset = (start_offset + offset) % range_len;
            AUTO_LOCAL_PORT_START + candidate_offset as u16
        })
        .collect()
}

fn auto_local_port_seed() -> Result<u16> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is set before the UNIX epoch")?
        .as_nanos();

    Ok(((u128::from(std::process::id()) + timestamp) % u128::from(AUTO_LOCAL_PORT_COUNT)) as u16)
}

fn rewrite_database_url(database_url: &Url, local_port: u16) -> Result<String> {
    let mut rewritten_database_url = database_url.clone();
    rewritten_database_url
        .set_host(Some("127.0.0.1"))
        .context("failed to rewrite database URL host for SSH tunnel")?;
    rewritten_database_url
        .set_port(Some(local_port))
        .map_err(|()| anyhow::anyhow!("failed to rewrite database URL port for SSH tunnel"))?;

    Ok(rewritten_database_url.to_string())
}

fn find_ssh_binary() -> Result<PathBuf> {
    which("ssh").context("could not find `ssh` in PATH; install OpenSSH and try again")
}

fn prepare_ssh_log_path() -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let pid = std::process::id();

    for attempt in 0..10u32 {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is set before the UNIX epoch")?
            .as_nanos();
        let path = temp_dir.join(format!("geni-ssh-{}-{}-{}.log", pid, timestamp, attempt));

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("failed to create SSH startup log at {}", path.display())
                });
            }
        }
    }

    bail!("failed to create a unique SSH startup log file")
}

fn cleanup_ssh_log(ssh_log_path: &Path) {
    let _ = fs::remove_file(ssh_log_path);
}

async fn try_start_ssh_tunnel(
    ssh_binary: &Path,
    plan: &TunnelPlan,
    local_port: u16,
    deadline: Instant,
    wait_timeout: usize,
) -> std::result::Result<Child, TunnelAttemptError> {
    let ssh_log_path = prepare_ssh_log_path().map_err(TunnelAttemptError::Fatal)?;
    let mut child = match spawn_ssh_tunnel(ssh_binary, plan, local_port, &ssh_log_path) {
        Ok(child) => child,
        Err(error) => {
            cleanup_ssh_log(&ssh_log_path);
            return Err(TunnelAttemptError::Fatal(error));
        }
    };

    let startup_result = wait_for_tunnel_attempt(
        &mut child,
        &ssh_log_path,
        local_port,
        deadline,
        wait_timeout,
    )
    .await;

    match startup_result {
        Ok(()) => {
            cleanup_ssh_log(&ssh_log_path);
            Ok(child)
        }
        Err(TunnelAttemptError::PortInUse) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            cleanup_ssh_log(&ssh_log_path);
            Err(TunnelAttemptError::PortInUse)
        }
        Err(TunnelAttemptError::Fatal(error)) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            cleanup_ssh_log(&ssh_log_path);
            Err(TunnelAttemptError::Fatal(error))
        }
    }
}

fn spawn_ssh_tunnel(
    ssh_binary: &Path,
    plan: &TunnelPlan,
    local_port: u16,
    ssh_log_path: &Path,
) -> Result<Child> {
    let mut command = Command::new(ssh_binary);
    command
        .args(plan.ssh_args.iter().map(OsString::as_os_str))
        .arg("-L")
        .arg(format!(
            "127.0.0.1:{}:{}:{}",
            local_port, plan.remote_host, plan.remote_port
        ))
        .arg("-o")
        .arg("LogLevel=DEBUG1")
        .arg("-E")
        .arg(ssh_log_path)
        .arg(&plan.ssh_destination)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    command
        .spawn()
        .with_context(|| format!("failed to start SSH tunnel using {}", ssh_binary.display()))
}

async fn wait_for_tunnel_attempt(
    child: &mut Child,
    ssh_log_path: &Path,
    local_port: u16,
    deadline: Instant,
    wait_timeout: usize,
) -> std::result::Result<(), TunnelAttemptError> {
    loop {
        let log_state =
            ssh_log_state(ssh_log_path, local_port).map_err(TunnelAttemptError::Fatal)?;
        let status = child
            .try_wait()
            .context("failed to check SSH tunnel process state")
            .map_err(TunnelAttemptError::Fatal)?;

        match (log_state, status) {
            (SshLogState::Listening, None) => return Ok(()),
            (SshLogState::PortInUse, Some(_)) => return Err(TunnelAttemptError::PortInUse),
            (_, Some(status)) => {
                return Err(TunnelAttemptError::Fatal(anyhow::anyhow!(
                    "SSH tunnel exited before it became ready with status {}",
                    status
                )));
            }
            _ => {}
        }

        if Instant::now() >= deadline {
            return Err(TunnelAttemptError::Fatal(anyhow::anyhow!(
                "SSH tunnel did not become ready within {} seconds",
                wait_timeout
            )));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

fn ssh_log_state(ssh_log_path: &Path, local_port: u16) -> Result<SshLogState> {
    let log_contents = match fs::read_to_string(ssh_log_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(SshLogState::None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read SSH startup log at {}",
                    ssh_log_path.display()
                )
            });
        }
    };

    if log_contents.contains(&format!(
        "Local forwarding listening on 127.0.0.1 port {}.",
        local_port
    )) {
        return Ok(SshLogState::Listening);
    }

    if log_contents.contains("Address already in use")
        || log_contents.contains(&format!("cannot listen to port: {}", local_port))
    {
        return Ok(SshLogState::PortInUse);
    }

    Ok(SshLogState::None)
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

    fn local_port_from_database_url(database_url: &str) -> u16 {
        Url::parse(database_url)
            .unwrap()
            .port()
            .expect("database URL should include a port")
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
                local_port: None,
                remote_host: None,
                remote_port: None,
            },
        )
        .unwrap();

        assert_eq!(plan.requested_local_port, None);
        assert_eq!(plan.remote_host, "db.internal");
        assert_eq!(plan.remote_port, 5432);
        assert_eq!(
            plan.ssh_args,
            vec![
                OsString::from("-o"),
                OsString::from("ExitOnForwardFailure=yes"),
                OsString::from("-N"),
            ]
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

        assert_eq!(plan.requested_local_port, Some(3307));
        assert_eq!(plan.remote_host, "mysql.internal");
        assert_eq!(plan.remote_port, 3308);
        assert_eq!(
            rewrite_database_url(&plan.database_url, 3307).unwrap(),
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
    fn test_local_port_candidates_wrap_auto_range() {
        let candidates = local_port_candidates(None, AUTO_LOCAL_PORT_COUNT - 1);

        assert_eq!(candidates.len(), AUTO_LOCAL_PORT_COUNT as usize);
        assert_eq!(candidates[0], AUTO_LOCAL_PORT_END);
        assert_eq!(candidates[1], AUTO_LOCAL_PORT_START);
        assert_eq!(candidates.last(), Some(&(AUTO_LOCAL_PORT_END - 1)));
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
            "#!/bin/sh\nprintf '%s\n' \"$@\" > \"{}\"\nlog_file=\"\"\nlocal_forward=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-E\" ]; then\n    log_file=\"$2\"\n    shift 2\n    continue\n  fi\n  if [ \"$1\" = \"-L\" ]; then\n    local_forward=\"$2\"\n    shift 2\n    continue\n  fi\n  shift\n done\nlocal_port=$(printf '%s' \"$local_forward\" | cut -d: -f2)\npython3 - <<'PY' \"$local_port\" \"$log_file\"\nimport socket, sys\nport = int(sys.argv[1])\nlog_file = sys.argv[2]\nserver = socket.socket()\nserver.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\nserver.bind(('127.0.0.1', port))\nserver.listen(1)\nwith open(log_file, 'a', encoding='utf-8') as fh:\n    fh.write(f'debug1: Local forwarding listening on 127.0.0.1 port {{port}}.\\n')\nwhile True:\n    conn, _ = server.accept()\n    conn.close()\nPY\n",
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
    async fn test_establish_tunnel_retries_auto_port_after_bind_conflict() -> Result<()> {
        if which("python3").is_err() {
            return Ok(());
        }

        let temp_dir = tempdir()?;
        let ssh_path = temp_dir.path().join("ssh");
        let attempt_count_path = temp_dir.path().join("attempt-count.txt");
        let attempted_ports_path = temp_dir.path().join("attempted-ports.txt");

        let script = format!(
            "#!/bin/sh\ncount_file=\"{}\"\nattempts_file=\"{}\"\nlog_file=\"\"\nlocal_forward=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-E\" ]; then\n    log_file=\"$2\"\n    shift 2\n    continue\n  fi\n  if [ \"$1\" = \"-L\" ]; then\n    local_forward=\"$2\"\n    shift 2\n    continue\n  fi\n  shift\n done\nlocal_port=$(printf '%s' \"$local_forward\" | cut -d: -f2)\ncount=1\nif [ -f \"$count_file\" ]; then\n  count=$(($(cat \"$count_file\") + 1))\nfi\nprintf '%s' \"$count\" > \"$count_file\"\nprintf '%s\n' \"$local_port\" >> \"$attempts_file\"\nif [ \"$count\" -eq 1 ]; then\n  printf 'bind [127.0.0.1]:%s: Address already in use\\n' \"$local_port\" >> \"$log_file\"\n  exit 1\nfi\npython3 - <<'PY' \"$local_port\" \"$log_file\"\nimport socket, sys\nport = int(sys.argv[1])\nlog_file = sys.argv[2]\nserver = socket.socket()\nserver.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\nserver.bind(('127.0.0.1', port))\nserver.listen(1)\nwith open(log_file, 'a', encoding='utf-8') as fh:\n    fh.write(f'debug1: Local forwarding listening on 127.0.0.1 port {{port}}.\\n')\nwhile True:\n    conn, _ = server.accept()\n    conn.close()\nPY\n",
            attempt_count_path.display(),
            attempted_ports_path.display()
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
                local_port: None,
                remote_host: None,
                remote_port: None,
            },
            2,
        )
        .await?;

        let attempted_ports: Vec<u16> = fs::read_to_string(&attempted_ports_path)?
            .lines()
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        assert_eq!(attempted_ports.len(), 2);
        assert_eq!(
            attempted_ports[1],
            if attempted_ports[0] == AUTO_LOCAL_PORT_END {
                AUTO_LOCAL_PORT_START
            } else {
                attempted_ports[0] + 1
            }
        );
        assert_eq!(
            local_port_from_database_url(&tunnel.database_url),
            attempted_ports[1]
        );

        tunnel.guard.shutdown().await?;

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial]
    async fn test_establish_tunnel_fails_for_explicit_port_conflict() -> Result<()> {
        let temp_dir = tempdir()?;
        let ssh_path = temp_dir.path().join("ssh");

        let script = "#!/bin/sh\nlog_file=\"\"\nlocal_forward=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-E\" ]; then\n    log_file=\"$2\"\n    shift 2\n    continue\n  fi\n  if [ \"$1\" = \"-L\" ]; then\n    local_forward=\"$2\"\n    shift 2\n    continue\n  fi\n  shift\n done\nlocal_port=$(printf '%s' \"$local_forward\" | cut -d: -f2)\nprintf 'bind [127.0.0.1]:%s: Address already in use\\n' \"$local_port\" >> \"$log_file\"\nexit 1\n";

        fs::write(&ssh_path, script)?;
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
        .expect("expected explicit local port conflict");

        assert_eq!(
            error.to_string(),
            "requested SSH tunnel local port 6432 is already in use"
        );

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial]
    async fn test_establish_tunnel_does_not_retry_non_bind_failure() -> Result<()> {
        let temp_dir = tempdir()?;
        let ssh_path = temp_dir.path().join("ssh");
        let attempts_path = temp_dir.path().join("attempts.txt");

        let script = format!(
            "#!/bin/sh\nattempts_file=\"{}\"\nlog_file=\"\"\nlocal_forward=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-E\" ]; then\n    log_file=\"$2\"\n    shift 2\n    continue\n  fi\n  if [ \"$1\" = \"-L\" ]; then\n    local_forward=\"$2\"\n    shift 2\n    continue\n  fi\n  shift\n done\nlocal_port=$(printf '%s' \"$local_forward\" | cut -d: -f2)\nprintf '%s\n' \"$local_port\" >> \"$attempts_file\"\nprintf 'Permission denied\\n' >> \"$log_file\"\nexit 12\n",
            attempts_path.display()
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

        let error = establish_tunnel(
            "postgres://user:secret@db.internal:5432/app?sslmode=disable".to_string(),
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: None,
                ssh_port: None,
                ssh_identity_file: None,
                local_port: None,
                remote_host: None,
                remote_port: None,
            },
            1,
        )
        .await
        .err()
        .expect("expected non-bind SSH failure");

        let attempts = fs::read_to_string(&attempts_path)?;
        assert_eq!(attempts.lines().count(), 1);
        assert!(error
            .to_string()
            .contains("SSH tunnel exited before it became ready"));

        Ok(())
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial]
    async fn test_establish_tunnel_waits_for_ssh_log_signal() -> Result<()> {
        let temp_dir = tempdir()?;
        let ssh_path = temp_dir.path().join("ssh");

        fs::write(&ssh_path, "#!/bin/sh\nsleep 5\n")?;
        let mut permissions = fs::metadata(&ssh_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&ssh_path, permissions)?;

        let existing_path = env::var_os("PATH").unwrap_or_default();
        let mut combined_path = OsString::from(temp_dir.path().as_os_str());
        combined_path.push(OsStr::new(":"));
        combined_path.push(existing_path);
        let _path_guard = EnvGuard::set_path("PATH", PathBuf::from(combined_path));

        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let listener_port = listener.local_addr()?.port();

        let error = establish_tunnel(
            "postgres://user:secret@db.internal:5432/app?sslmode=disable".to_string(),
            SshTunnelConfig {
                ssh_host: "bastion".to_string(),
                ssh_user: None,
                ssh_port: None,
                ssh_identity_file: None,
                local_port: Some(listener_port),
                remote_host: None,
                remote_port: None,
            },
            1,
        )
        .await
        .err()
        .expect("expected SSH tunnel startup timeout");

        drop(listener);

        assert_eq!(
            error.to_string(),
            "SSH tunnel did not become ready within 1 seconds"
        );

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
            "#!/bin/sh\necho $$ > \"{}\"\nlog_file=\"\"\nlocal_forward=\"\"\nwhile [ $# -gt 0 ]; do\n  if [ \"$1\" = \"-E\" ]; then\n    log_file=\"$2\"\n    shift 2\n    continue\n  fi\n  if [ \"$1\" = \"-L\" ]; then\n    local_forward=\"$2\"\n    shift 2\n    continue\n  fi\n  shift\n done\nlocal_port=$(printf '%s' \"$local_forward\" | cut -d: -f2)\npython3 - <<'PY' \"$local_port\" \"$log_file\"\nimport socket, sys\nport = int(sys.argv[1])\nlog_file = sys.argv[2]\nserver = socket.socket()\nserver.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)\nserver.bind(('127.0.0.1', port))\nserver.listen(1)\nwith open(log_file, 'a', encoding='utf-8') as fh:\n    fh.write(f'debug1: Local forwarding listening on 127.0.0.1 port {{port}}.\\n')\nwhile True:\n    conn, _ = server.accept()\n    conn.close()\nPY\n",
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
