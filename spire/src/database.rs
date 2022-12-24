use anyhow::{bail, Result};

use sqlx::{migrate::Migrator, postgres::PgPoolOptions, Pool, Postgres, Transaction};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

pub struct Database(pub Pool<Postgres>);

impl Database {
    pub async fn new(dsn: String) -> Result<Database> {
        let database = PgPoolOptions::new()
            .max_connections(150)
            .connect(&dsn)
            .await?;
        Ok(Self(database))
    }
    pub async fn migrate(&self) -> Result<()> {
        let migrations_dir = PathBuf::from("./migrations");
        if !migrations_dir.is_dir() || !migrations_dir.exists() {
            bail!("Migrations path is not a directory");
        }
        let migrator = Migrator::new(migrations_dir).await?;
        migrator.run(&self.0).await?;
        Ok(())
    }
}
