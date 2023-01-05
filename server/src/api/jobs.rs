use actix_web::{
    get,
    web::{self},
    Error, HttpRequest, HttpResponse,
};

use anyhow::Result;

use crate::{
    models::{Job, JobLog, Repos},
    Spire,
};

#[get("/jobs/{job}/logs")]
async fn get_job_logs(
    app: web::Data<Spire>,
    job: web::Path<i64>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let job_id = job.into_inner();

    let Ok(logs) = sqlx::query_as::<_, JobLog>(r#"SELECT * FROM job_logs WHERE job = $1 ORDER BY id ASC"#)
    .bind(job_id)
    .fetch_all(&app.database.0)
    .await else {
        return Ok(HttpResponse::InternalServerError().finish());
    };

    let mut html = format!(
        r#"
    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta http-equiv="X-UA-Compatible" content="IE=edge">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Document</title>
            <link href="./styles.css" rel="stylesheet">
        </head>
        <body>
            <h1>Showing logs for job #{}</h1>
            <ul style="list-style: none; background-color: black; color: white;">
         
    "#,
        job_id
    );

    for log in logs {
        let (name, style) = match log.status {
            0 => ("ok", "background-color: green; color: black;"),
            2 => ("err", "background-color: red; color; black"),
            _ => ("unkown", "background-color: black; color: white;"),
        };
        let status = format!(r#"[<span style="{style}">{name}</span>]"#);
        html.push_str(&format!(
            r#"<li>[{}] {} {}</li>"#,
            log.step, status, log.output
        ));
    }
    html.push_str("</ul></body></html>");
    Ok(HttpResponse::Ok().body(html))
}

#[get("/jobs/{repo}")]
async fn get_repo_jobs(
    app: web::Data<Spire>,
    repo: web::Path<i64>,
    _req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let repo_id = repo.into_inner();
    let Ok(repo) = sqlx::query_as::<_, Repos>(r#"SELECT * FROM repos WHERE id = $1"#)
    .bind(repo_id)
    .fetch_one(&app.database.0)
    .await else {
        return Ok(HttpResponse::InternalServerError().finish());
    };

    let Ok(jobs) = sqlx::query_as::<_, Job>(r#"SELECT * FROM job WHERE repo = $1"#)
    .bind(repo_id)
    .fetch_all(&app.database.0)
    .await else {
        return Ok(HttpResponse::InternalServerError().finish());
    };

    let mut html = format!(
        r#"
    <!DOCTYPE html>
    <html lang="en">
        <head>
            <meta charset="UTF-8">
            <meta http-equiv="X-UA-Compatible" content="IE=edge">
            <meta name="viewport" content="width=device-width, initial-scale=1.0">
            <title>Document</title>
        </head>
        <body>
            <h1>Showing all jobs for repository {}/{} (#{})</h1>
            <a href="/repos">View All Repos</a>
            <ul>
         
    "#,
        repo.owner, repo.name, repo.id
    );

    for job in jobs {
        let (name, style) = match job.status {
            0 => ("queued", "background-color: yellow; color: black;"),
            1 => ("running", "background-color: orange; color: black;"),
            2 => ("failed", "background-color: red; color; black"),
            3 => ("success", "background-color: green; color: black"),
            _ => ("unkown", "background-color: black; color: white;"),
        };
        let status = format!(r#"[<span style="{style}">{name}</span>]"#);
        html.push_str(&format!(
            r#"<li>[{id} - Started By: {}] [Ran on {}] {} - <a href="/jobs/{id}/logs">[View Logs]</a></li>"#,
            job.triggered_by,
            job.assigned_runner,
            status,
            id = job.id,
        ));
    }
    html.push_str("</ul></body></html>");
    Ok(HttpResponse::Ok().body(html))
}
