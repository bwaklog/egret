pub mod client;
pub mod config;

use crate::client::MatrixClient;
use tracing::info;
use tracing_subscriber::{EnvFilter, FmtSubscriber, fmt::time};


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("rot=info,matrix_sdk=error"));

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_timer(time::ChronoLocal::rfc_3339())
        .with_target(true)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    let config = config::RotConfig::load_config(None)?;
    config.source_env()?;

    info!("{:#?}", &config);
    let client = MatrixClient::init(config.client).await?;
    client.login_and_sync().await?;

    // info!("Adding event handler for room messages");

    // let room_id = config
    //     .rooms
    //     .iter()
    //     .nth(0)
    //     .unwrap()
    //     .room_id
    //     .as_str()
    //     .try_into()?;
    // client
    //     .client
    //     .add_room_event_handler(room_id, |_: SyncRoomMessageEvent| async move {
    //         info!("got a message");
    //     });

    // client.add_room_event_handler(room_id, |ev: SyncRoomMessageEvent| async move {
    //     if let SyncRoomMessageEvent::Original(OriginalSyncRoomMessageEvent {
    //         content,
    //         sender,
    //         ..
    //     }) = ev
    //     {
    //         // println!("received message: {content:#?} from sender {sender}",);
    //         if let MessageType::Text(content) = content.msgtype {
    //             println!("message from {sender}: {}", content.body);
    //         }
    //     }
    // });
    // println!("added event handler for room {room_id}");

    // // get room information
    // let room = client
    //     .get_room(room_id)
    //     .expect("failed to get room for room_id");
    // println!("room name: {:?}", room.display_name().await.ok());

    // let rooms = client.client.joined_rooms();
    // println!("number of joined rooms: {}", rooms.len());
    // for room in rooms {
    //     println!("room_id={}", room.room_id());
    //     println!("name={:?}", room.display_name().await.ok());
    // }

    // let _ = tokio::signal::ctrl_c().await;

    Ok(())
}
