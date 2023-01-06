use super::Job;
use anyhow::{bail, Result};
use sqlx::{migrate::Migrator, postgres::PgPoolOptions, Pool, Postgres};
use std::path::PathBuf;

#[repr(C)]
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

    pub async fn get_jobs_paginated(
        &self,
        page: Option<i64>,
        per_page: Option<i64>,
    ) -> Result<Vec<Job>> {
        let page = if page.is_none() || page.unwrap() == 1 {
            // From my testing I think its better to just return 0 for the first page and limit it by the per_page option..
            0
        } else {
            page.unwrap() * per_page.unwrap_or(5)
        };
        let jobs =
            sqlx::query_as::<_, Job>(r#"SELECT * FROM job WHERE id > $1 ORDER BY id ASC LIMIT $2"#)
                .bind(page)
                .bind(per_page.unwrap_or(5))
                .fetch_all(&self.0)
                .await?;
        Ok(jobs)
    }
}
