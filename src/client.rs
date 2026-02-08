use std::{
    fs::OpenOptions,
    io::{BufReader, BufWriter},
    time::Duration,
};

use crate::config::MatrixClientConfig;
use anyhow::{Ok, anyhow};
use matrix_sdk::{
    Client,
    authentication::matrix::MatrixSession,
    config::SyncSettings,
    ruma::{OwnedDeviceId, OwnedUserId},
};
use serde::{Deserialize, Serialize};
use tracing::{error, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    access_token: String,
    refresh_token: Option<String>,
    user_id: OwnedUserId,
    device_id: OwnedDeviceId,
}

impl From<Session> for MatrixSession {
    fn from(value: Session) -> Self {
        MatrixSession {
            meta: matrix_sdk::SessionMeta {
                user_id: value.user_id,
                device_id: value.device_id,
            },
            tokens: matrix_sdk::SessionTokens {
                access_token: value.access_token,
                refresh_token: value.refresh_token,
            },
        }
    }
}

impl From<MatrixSession> for Session {
    fn from(value: MatrixSession) -> Self {
        Session {
            access_token: value.tokens.access_token,
            refresh_token: value.tokens.refresh_token,
            user_id: value.meta.user_id,
            device_id: value.meta.device_id,
        }
    }
}

pub struct MatrixClient {
    pub client: matrix_sdk::Client,

    user_id: OwnedUserId,
    password: String,
    recovery_code: String,

    config: MatrixClientConfig,
}

impl MatrixClient {
    // handle login and sync as well
    pub async fn init(config: MatrixClientConfig) -> anyhow::Result<Self> {
        let user_id: OwnedUserId = config.clone().user_id.try_into()?;
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
            config: config,
        };

        Ok(config)
    }

    async fn spawn_sync_task(&self) -> anyhow::Result<()> {
        let client = self.client.clone();
        let post_client = self.client.clone();
        let (sync_tx, sync_rx) = tokio::sync::oneshot::channel::<anyhow::Result<()>>();

        _ = tokio::spawn(async move {
            let _ = sync_tx.send(
                client
                    .sync_once(SyncSettings::default().timeout(Duration::from_secs(20)))
                    .await
                    .map(|_| ())
                    .map_err(Into::into),
            );

            loop {
                if let Err(e) = client.sync(SyncSettings::default()).await {
                    error!("background sync failed: {e}");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        });

        sync_rx.await??;
        let uid = post_client
            .user_id()
            .ok_or(anyhow!("failed to get user id after sync"))?;
        info!("uid" = uid.to_string(), "background sync has started");

        Ok(())
    }

    pub async fn login(&self) -> anyhow::Result<()> {
        // if we have a session file
        if self.config.sessions_file.exists() {
            let session = self.load_session()?;
            let matrix_session = MatrixSession::from(session);
            self.client
                .restore_session(matrix_session)
                .await
                .map_err(|e| anyhow!("Failed to restore session: {e}"))?;

            self.client
                .encryption()
                .recovery()
                .recover(&self.recovery_code)
                .await?;

            info!("Encryption recovery successful for {}", self.user_id);

            self.spawn_sync_task().await?;
            Ok(())
        } else {
            self.login_and_sync().await?;
            Ok(())
        }
    }

    async fn login_and_sync(&self) -> anyhow::Result<()> {
        let client = &self.client;
        let resp = client
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

        let session = MatrixSession::from(&resp);
        self.save_session(session)?;

        self.spawn_sync_task().await?;

        Ok(())
    }

    #[allow(unused)]
    fn save_session(&self, session: MatrixSession) -> anyhow::Result<()> {
        let file = OpenOptions::new()
            .read(false)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.config.sessions_file)?;
        let writer = BufWriter::new(file);
        let session = Session::from(session);
        serde_json::to_writer(writer, &session)
            .map_err(|e| anyhow!("failed to write session to file: {e}"))?;
        Ok(())
    }

    #[allow(unused)]
    fn load_session(&self) -> anyhow::Result<Session> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.config.sessions_file)?;
        let reader = BufReader::new(file);
        let session = serde_json::from_reader(reader)
            .map_err(|e| anyhow!("failed to read session from file: {e}"))?;
        Ok(session)
    }
}
