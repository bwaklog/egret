# Egret 🪿
> Configurable matrix client, because I don't want to work with the Whatsapp API (I've been banned)

## Configuration

1. Setup `config.json`
  - `client.user_id`: Your matrix user ID, e.g. `@username:beeper.com`
  - `client.sqlite_store` (optional): Local sqlite store for storing matrix client data
  - `client.sessions_file`: For session persistence (refresh and access tokens)
  - `rooms`: List of rooms to join, each with `room_id` and `name`
  
  An example is shown below

  ```json
  {
    "client": {
      "user_id": "@username:beeper.com",
      "sqlite_store": "path_to_sqlite_db",
      "sessions_file": "sessions.json"
    },
    "rooms": [
      {
        "room_id": "!chat:beeper.local",
        "name": "personal chat"
      }
    ]
  }
  ```

2. Setup the `.env` file
  - `MATRIX_USER_ID`: Your matrix user ID, e.g. `@username:beeper.com` (to be removed)
  - `MATRIX_PASSWORD`: Your matrix password
  - `BEEPER_RECOVERY_CODE`: required as the recovery key for the client
  - `TURSO_DB_URL`: URL for the Turso database (e.g. `libsql://`)
  - `TURSO_AUTH_TOKEN`: Authentication token for the Turso database
  
  > Plans to push content to a turso db, hence the creds are required without any purpose as of now.
  
## Build and run

```fish
git clone https://github.com/bwaklog/egret.git
cd egret

cargo build -r
./target/release/egret
```
