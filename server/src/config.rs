use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Github {
    pub webhook_secret: String,
    pub key_path: String,
    pub app_id: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Plugin {
    pub enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub database: String,
    pub server: Option<Server>,
    pub github: Github,
    pub plugins_directory: Option<String>,
    pub plugins: Option<HashMap<String, Plugin>>,
}

pub const CONFIG: Lazy<Config> = Lazy::new(|| {
    toml::from_str::<Config>(&std::fs::read_to_string("./server/Config.toml").unwrap()).unwrap()
});
// seconds
pub const HEARTBEAT_INTERVAL: u64 = 10;
// seconds
pub const CLIENT_TIMEOUT: u64 = 10;
