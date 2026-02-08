use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    io::BufReader,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomConfig {
    pub room_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixClientConfig {
    pub user_id: String,
    #[serde(default = "default_sqlite_store")]
    pub sqlite_store: String,
    pub sessions_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EgretConfig {
    pub rooms: Vec<RoomConfig>,
    pub client: MatrixClientConfig,

    #[serde(default = "default_env_file")]
    pub env_file: String,
}

// I don't like this
fn default_env_file() -> String {
    ".env".to_string()
}

fn default_sqlite_store() -> String {
    "./sqlite_store".to_string()
}

impl EgretConfig {
    pub fn load_config(path: Option<String>) -> anyhow::Result<Self> {
        let mut config_file = std::fs::OpenOptions::new()
            .read(true)
            .create(false)
            .write(false)
            .truncate(false)
            .open(path.unwrap_or("config.json".to_string()))
            .map_err(|e| anyhow!("failed to open config file, not found: {e}"))?;

        let reader = BufReader::new(&mut config_file);

        let config: Self =
            serde_json::from_reader(reader).map_err(|e| anyhow!("parsing failed: {e}"))?;
        Ok(config)
    }

    pub fn source_env(&self) -> anyhow::Result<()> {
        let path = Path::new(&self.env_file);
        path.exists()
            .then_some(())
            .ok_or(anyhow!("env file not found {}", &self.env_file))?;

        dotenvy::from_path(path).map_err(|e| anyhow!("failed to load env: {e}"))?;

        let req: HashSet<String> = HashSet::from([
            "MATRIX_USER_ID".to_string(),
            "MATRIX_PASSWORD".to_string(),
            "BEEPER_RECOVERY_CODE".to_string(),
            "TURSO_DB_URL".to_string(),
            "TURSO_AUTH_TOKEN".to_string(),
        ]);
        let got: HashSet<String> = std::env::vars().map(|(k, _)| k).collect();

        let diff = req.difference(&got).collect::<Vec<_>>();
        diff.len()
            .eq(&0)
            .then_some(())
            .ok_or(anyhow!("missing values in .env: {:?}", diff))?;

        Ok(())
    }
}
