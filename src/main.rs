pub mod client;
pub mod config;

use crate::client::MatrixClient;
use anyhow::anyhow;
use matrix_sdk::ruma::events::{
    OriginalSyncMessageLikeEvent,
    room::message::{MessageType, RoomMessageEventContent, SyncRoomMessageEvent},
};
use tracing::info;
use tracing_subscriber::{EnvFilter, FmtSubscriber, fmt::time};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("egret=info,matrix_sdk=error"));

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_timer(time::ChronoLocal::rfc_3339())
        .with_target(true)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let config = config::EgretConfig::load_config(None)?;
    config.source_env()?;
    info!("sourced config and env variables");

    let matrix_client = MatrixClient::init(config.client).await?;
    matrix_client.login().await?;

    // info!("Adding event handler for room messages");

    config.rooms.iter().for_each(|room| {
        let room_config = room.clone();
        let innser_clinet = matrix_client.client.clone();
        let inner_room_config = room_config.clone();
        matrix_client.client.add_room_event_handler(
            room_config.room_id.as_str().try_into().unwrap(),
            |ev: SyncRoomMessageEvent| async move {
                if let SyncRoomMessageEvent::Original(OriginalSyncMessageLikeEvent {
                    content,
                    sender,
                    event_id,
                    ..
                }) = ev
                {
                    if let MessageType::Text(content) = content.msgtype {
                        info!(
                            "sender" = sender.as_str(),
                            "room name" = room_config.name,
                            "message" = content.body
                        );
                        if content.body.starts_with("@egret") {
                            info!(
                                "event_id" = event_id.as_str(),
                                "sender" = sender.to_string(),
                                "is a bot command"
                            );
                            let room = innser_clinet
                                .get_room(inner_room_config.room_id.as_str().try_into().unwrap())
                                .ok_or(anyhow!(
                                    "failed to get a room with id {}",
                                    inner_room_config.room_id
                                ))
                                .unwrap();
                            info!(
                                "room is encrypted: {}",
                                room.encryption_state().is_encrypted()
                            );

                            let msg = RoomMessageEventContent::text_plain("[bot] pong");
                            let response = room.send(msg).await.unwrap();
                            info!(
                                "response event id" = response.event_id.as_str(),
                                "response sent"
                            );
                        }
                    }
                }
            },
        );
    });

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
