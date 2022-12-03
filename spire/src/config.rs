use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Server {
    pub port: u16,
    pub host: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub database: String,
    pub server: Option<Server>,
}
