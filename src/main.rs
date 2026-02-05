use anyhow::Ok;
use dotenvy::dotenv;
use matrix_sdk::{
    Client,
    config::SyncSettings,
    ruma::{events::room::message::SyncRoomMessageEvent, user_id},
};
use std::env;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().ok();

    let matrix_password =
        env::var("MATRIX_PASSWORD").expect("MATRIX_PASSWORD missing for client in .env");

    let bwaklog = user_id!("@bwaklog:beeper.com");
    let client = Client::builder()
        .server_name(bwaklog.server_name())
        .build()
        .await?;

    client
        .matrix_auth()
        .login_username(bwaklog, matrix_password.as_str())
        .send()
        .await?;

    info!("created matrix client, logged in sucessfully");

    client.add_event_handler(|ev: SyncRoomMessageEvent| async move {
        info!("recieved message: {:?}", ev);
    });
    client.sync(SyncSettings::default()).await?;

    Ok(())
}
