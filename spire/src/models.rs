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
