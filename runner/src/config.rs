use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Spire {
    pub host: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub name: String,
    pub password: String,
    pub spire: Spire,
}
