use super::DatabaseDriver;
use crate::config;
use anyhow::{bail, Result};

pub async fn wait_for_database(client: &mut dyn DatabaseDriver) -> Result<()> {
    let wait_timeout = config::wait_timeout();

    let mut count = 0;
    loop {
        if count > wait_timeout {
            bail!("Database is not ready");
        }

        if client.ready().await.is_ok() {
            break;
        }

        count += 1;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}
