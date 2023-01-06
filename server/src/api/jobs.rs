use actix_web::{
    get,
    web::{self},
    HttpRequest, HttpResponse, Responder,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    models::{Job, Repos},
    Spire,
    errors::Error
};

#[derive(Serialize, Deserialize)]
pub struct GetJobsQuery {
    pub repo: Option<i64>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[get("/jobs")]
async fn get_jobs(
    app: web::Data<Spire>,
    query: web::Query<GetJobsQuery>,
    _req: HttpRequest,
) -> Result<impl Responder, Error> {
    let query = query.into_inner();

    let jobs = match app
        .database
        .get_jobs_paginated(query.page, query.per_page)
        .await
    {
        Ok(jobs) => jobs,
        Err(err) => {
            return Err(Error::internal_server_error(String::from("Failed to get jobs related to your search.")));
        }
    };
    Ok(HttpResponse::Ok().json(jobs))
}

#[get("/jobs/{id}")]
async fn get_specific_job(
    app: web::Data<Spire>,
    id: web::Path<i64>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let job_id = id.into_inner();
    let Ok(job) = sqlx::query_as::<_, Job>(r#"SELECT * FROM job WHERE id = $1"#)
    .bind(job_id)
    .fetch_one(&app.database.0)
    .await else {
        return Err(Error::internal_server_error(String::from("Failed to get the requested job.")));
    };

    let Ok(repo) = sqlx::query_as::<_, Repos>(r#"SELECT * FROM repos WHERE id = $1"#)
    .bind(job.repo)
    .fetch_one(&app.database.0)
    .await else {
        return Err(Error::internal_server_error(String::from("Failed to get a job's repo.")));
    };

    Ok(HttpResponse::Ok().json(json!({
        "id": job.id,
        "start": job.start,
        "end": job.end,
        "assigned_runner": job.assigned_runner,
        "triggered_by": job.triggered_by,
        "status": job.status,
        "repo": repo,
    })))
}
