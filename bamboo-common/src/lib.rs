use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Serialize, Deserialize, Debug, FromRow, Clone)]
pub struct Runners {
    pub name: String,
    pub id: i64,
    pub local_path: String,
    pub created_at: NaiveDateTime,
}

#[derive(Serialize, Deserialize, Debug, FromRow, Clone)]
pub struct Repos {
    pub id: i64,
    pub gh_id: i64,
    pub owner: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, FromRow, Clone)]
pub struct Job {
    pub id: i64,
    pub assigned_runner: String,
    pub status: i64,
    pub repo: i64,
}

pub mod websocket {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    #[serde(untagged)]
    pub enum Messages {
        GetJobRepo { job: i64, repo: i64 },
    }
}
