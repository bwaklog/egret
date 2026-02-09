pub mod client;
pub mod config;
pub mod utils;

use crate::client::MatrixClient;
use anyhow::anyhow;
use base64::prelude::*;
use matrix_sdk::{
    media::MediaRequestParameters,
    ruma::events::{
        AnySyncMessageLikeEvent, OriginalSyncMessageLikeEvent, SyncMessageLikeEvent,
        room::message::{
            MessageType, Relation, ReplyMetadata, RoomMessageEventContent, SyncRoomMessageEvent,
        },
    },
};
use tokio::fs;
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

    fs::create_dir_all(config.client.image_store.clone())
        .await
        .map_err(|e| anyhow!("unable to create media store dir {e}"))?;

    let matrix_client = MatrixClient::init(config.client).await?;
    matrix_client.login().await?;

    // info!("Adding event handler for room messages");

    config.rooms.iter().for_each(|room| {
        let room_config = room.clone();
        let inner_clinet = matrix_client.client.clone();
        let inner_room_config = room_config.clone();
        matrix_client.client.add_room_event_handler(
            room_config.room_id.as_str().try_into().unwrap(),
            |ev: SyncRoomMessageEvent| async move {
                // info!("content = {:#?}", ev);

                if let SyncRoomMessageEvent::Original(OriginalSyncMessageLikeEvent {
                    content,
                    sender,
                    event_id,
                    ..
                }) = ev
                {
                    if let Some(Relation::Reply { in_reply_to }) = content.relates_to {
                        // info!(
                        //     "sender" = sender.as_str(),
                        //     "room name" = room_config.name,
                        //     "message is a reply to event id" = in_reply_to.event_id.as_str()
                        // );
                        let room = inner_clinet
                            .get_room(inner_room_config.room_id.as_str().try_into().unwrap())
                            .unwrap();
                        let event = room
                            .load_or_fetch_event(&in_reply_to.event_id, None)
                            .await
                            .unwrap();

                        // info!("event: {:#?}", event);

                        if let Ok(matrix_sdk::ruma::events::AnySyncTimelineEvent::MessageLike(
                            AnySyncMessageLikeEvent::RoomMessage(SyncMessageLikeEvent::Original(
                                OriginalSyncMessageLikeEvent {
                                    content, sender, ..
                                },
                            )),
                        )) = event.raw().deserialize()
                        {
                            info!(
                                // "event_id" = event_id.to_string(),
                                "sender" = sender.to_string(),
                                "content" = content.body(),
                                "room name" = room_config.name,
                                "in reply to message"
                            );
                        }
                    } else if let MessageType::Text(text_content) = content.msgtype {
                        info!(
                            "sender" = sender.as_str(),
                            "room name" = room_config.name,
                            "message" = text_content.body
                        );
                        if text_content.body.starts_with("@egret") {
                            // info!(
                            //     "event_id" = event_id.as_str(),
                            //     "sender" = sender.to_string(),
                            //     "is a bot command"
                            // );
                            let room = inner_clinet
                                .get_room(inner_room_config.room_id.as_str().try_into().unwrap())
                                .ok_or(anyhow!(
                                    "failed to get a room with id {}",
                                    inner_room_config.room_id
                                ))
                                .unwrap();
                            // info!(
                            //     "room is encrypted: {}",
                            //     room.encryption_state().is_encrypted()
                            // );

                            let msg = RoomMessageEventContent::text_plain("[bot] pong");
                            let reply_metadata = ReplyMetadata::new(&event_id, &sender, None);
                            let reply = msg.make_reply_to(
                                reply_metadata,
                                matrix_sdk::ruma::events::room::message::ForwardThread::No,
                                matrix_sdk::ruma::events::room::message::AddMentions::Yes,
                            );
                            let response = room.send(reply).await.unwrap();
                            info!(
                                "response event id" = response.event_id.as_str(),
                                "response sent"
                            );
                        }
                    } else if let MessageType::Image(image_message_content) = content.msgtype {
                        info!(
                            "sender" = sender.as_str(),
                            "room name" = room_config.name,
                            "image source" = image_message_content.filename(),
                            "recieved an image message"
                        );
                        let media_source = image_message_content.source;
                        let media_request = MediaRequestParameters {
                            source: media_source,
                            format: matrix_sdk::media::MediaFormat::File,
                        };

                        let mime_type = image_message_content
                            .info
                            .unwrap()
                            .mimetype
                            .unwrap()
                            .parse()
                            .unwrap();
                        let filename = image_message_content.filename.unwrap();

                        let filename_ident_format = format!(
                            "{}-{}-{}-{}",
                            inner_room_config.room_id,
                            sender.clone().to_string(),
                            event_id.as_str(),
                            chrono::Local::now().to_rfc3339(),
                        );

                        let filename_ident_part = BASE64_STANDARD.encode(filename_ident_format);
                        let filename = Some(format!("{}-{}", filename_ident_part, filename));

                        let media = inner_clinet
                            .media()
                            .get_media_file(
                                &media_request,
                                filename.clone(),
                                &mime_type,
                                false,
                                Some("tmp".to_string()),
                            )
                            .await
                            .unwrap();
                        let file_path = std::path::Path::new("./tmp");
                        let file_path = file_path.join(filename.unwrap());
                        let persist_result = media
                            .persist(file_path.as_path())
                            .map_err(|e| anyhow!("failed to persist file: {e}"))
                            .unwrap();

                        info!("file persisted and synced to: {:#?}", persist_result);
                    }
                }
            },
        );
    });

    let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
