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
    pub install: i64,
    pub owner: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, FromRow, Clone)]
pub struct JobLog {
    pub id: i64,
    pub job: i64,
    pub step: String,
    pub status: i64,
    pub output: String,
}

#[derive(Serialize, Deserialize, Debug, FromRow, Clone)]
/// From @ Github
pub struct AccessToken {
    pub token: String,
}

#[derive(Serialize, Deserialize, Debug, FromRow, Clone)]
pub struct Installations {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug, FromRow, Clone)]
pub struct Job {
    pub id: i64,
    pub assigned_runner: String,
    pub status: i64,
    pub repo: i64,
}

pub mod ci_files;
pub mod websocket;
