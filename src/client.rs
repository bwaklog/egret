use std::time::Duration;

use crate::config::{MatrixClientConfig};
use anyhow::{Ok, anyhow};
use matrix_sdk::{
    Client,
    config::SyncSettings,
    ruma::OwnedUserId,
};
use tracing::{info, error};

pub struct MatrixClient {
    pub client: matrix_sdk::Client,

    user_id: OwnedUserId,
    password: String,
    recovery_code: String,
}

impl MatrixClient {
    // handle login and sync as well
    pub async fn init(config: MatrixClientConfig) -> anyhow::Result<Self> {
        let user_id: OwnedUserId = config.user_id.try_into()?;
        let client = Client::builder()
            .server_name(user_id.server_name())
            .sqlite_store(&config.sqlite_store, None)
            .build()
            .await?;

        let config = MatrixClient {
            client,
            user_id,
            password: std::env::var("MATRIX_PASSWORD")
                .map_err(|_| anyhow!("MATRIX_PASSWORD missing from .env"))?,
            recovery_code: std::env::var("BEEPER_RECOVERY_CODE")
                .map_err(|_| anyhow!("BEEPER_RECOVERY_CODE missing from .env"))?,
        };

        Ok(config)
    }

    pub async fn login_and_sync(&self) -> anyhow::Result<()> {
        let client = &self.client;
        let sync_client = client.clone();
        let (sync_tx, sync_rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();
        client
            .matrix_auth()
            .login_username(&self.user_id, &self.password)
            .send()
            .await?;
        info!("Logged in successfully as {}", self.user_id);

        client
            .encryption()
            .recovery()
            .recover(&self.recovery_code)
            .await?;
        info!("Encryption recovery successful for {}", self.user_id);

        _ = tokio::spawn(async move {
            let _ = sync_tx.send(
                sync_client
                    .sync_once(SyncSettings::default().timeout(Duration::from_secs(20)))
                    .await
                    .map(|_| ())
                    .map_err(Into::into),
            );

            loop {
                if let Err(e) = sync_client.sync(SyncSettings::default()).await {
                    error!("background sync failed: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });

        sync_rx.await??;
        info!("Initial sync completed");

        Ok(())
    }

    #[allow(unused)]
    fn save_session(&self) -> anyhow::Result<()> {
        unimplemented!()
    }

    #[allow(unused)]
    fn load_session(&self) -> anyhow::Result<()> {
        unimplemented!()
    }
}
