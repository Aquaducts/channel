use actix::Actor;

use actix_web::{
    get, post,
    web::scope,
    web::{self},
    App, Error, HttpRequest, HttpResponse, HttpServer,
};
use actix_web_actors::ws;
use anyhow::Result;
use octocrab::models::{
    events::payload::PushEventPayload, orgs::Organization, repos::GitUser, User,
};
use serde::{Deserialize, Serialize};
use spiar::{
    api::{
        github::manage_new_install,
        jobs::{get_job_logs, get_repo_jobs},
    },
    config::CONFIG,
    database::Database,
    messages::JobRequest,
    models::{Job, Repos, Runners},
    plugins::PLUGINS,
    socket::SocketSession,
    Connections, Spire,
};
use sqlx::FromRow;
use std::{collections::HashMap, fs::read_to_string, pin::Pin, sync::Arc};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectRequest {
    pub name: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunnerConfigFile {
    pub name: String,
    pub password: Option<String>,
}

#[get("/repos")]
async fn get_repos(app: web::Data<Spire>, _req: HttpRequest) -> Result<HttpResponse, Error> {
    let Ok(repos) = sqlx::query_as::<_, common::Repos>(r#"SELECT * FROM repos"#)
    .fetch_all(&app.database.0)
    .await else {
        return Ok(HttpResponse::InternalServerError().finish());
    };

    let mut html = String::from(
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
         <ul>
         
    "#,
    );

    for repo in repos {
        html.push_str(&format!(
            r#"<li><a href="/jobs/{id}">[{id}] - {}/{}</a></li>"#,
            repo.owner,
            repo.name,
            id = repo.id
        ))
    }
    html.push_str("</ul></body></html>");

    Ok(HttpResponse::Ok().body(html))
}

#[get("/ws")]
async fn create_ws_session(
    ws: web::Data<Spire>,
    params: web::Query<ConnectRequest>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (name, password) = (params.0.name, params.0.password);
    let Ok(possible_runner) = sqlx::query_as::<_, Runners>(r#"SELECT * FROM runners WHERE name = $1"#)
        .bind(&name)
        .fetch_one(&ws.database.0)
        .await else {
            println!("L");
            return Ok(HttpResponse::InternalServerError().finish());
        };

    let Ok(config_file) = read_to_string(format!("{}/Config.toml", possible_runner.local_path)) else {
        return Ok(HttpResponse::InternalServerError().finish());
    };
    let Ok(deserialized) = toml::from_str::<RunnerConfigFile>(&config_file) else {
        return Ok(HttpResponse::InternalServerError().finish());
    };
    if let Some(runner_pass) = deserialized.password {
        if password != runner_pass {
            return Ok(HttpResponse::Forbidden().finish());
        }
    }

    let new_connection = SocketSession {
        app: Pin::new(&ws.connections).get_ref().clone(),
        runner: name,
        database: ws.database.clone(),
    };
    let resp = ws::start(new_connection, &req, stream)?;
    Ok(resp)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestJson {
    pub name: String,
    pub owner: String,
}

#[derive(Debug, FromRow, Clone)]
pub struct JobCount(i64);

pub async fn queue_job(
    //app: Arc<RealApp>,
    ws: Arc<Spire>,
    repo_name: String,
    repo_owner: String,
    creator: String,
    runner: String,
    guard_against_queue: bool,
) -> Result<()> {
    let repo = sqlx::query_as::<_, Repos>(r#"SELECT * FROM repos WHERE name = $1 AND owner = $2"#)
        .bind(&repo_name)
        .bind(&repo_owner)
        .fetch_one(&ws.database.0)
        .await?;

    // do some repo magic when github notifications start to work.
    let new_job = sqlx::query_as::<_, Job>(
        r#"INSERT INTO job(assigned_runner, repo, triggered_by) VALUES($1,$2,$3) RETURNING *"#,
    )
    .bind(&runner)
    .bind(repo.id)
    .bind(&creator)
    .fetch_one(&ws.database.0)
    .await?;

    if !guard_against_queue {
        ws.connections
            .send(JobRequest {
                runner,
                job: new_job,
            })
            .await?;
        return Ok(());
    }

    let all_possible_queued_jobs = sqlx::query_as::<_, JobCount>(
        r#"SELECT count(*) FROM job WHERE status = 0 AND assigned_runner = $1"#,
    )
    .bind(&runner)
    .fetch_one(&ws.database.0)
    .await?;

    if all_possible_queued_jobs.0 <= 1 {
        println!("sent");
        ws.connections
            .send(JobRequest {
                runner,
                job: new_job,
            })
            .await?;
    }

    Ok(())
}

/// This is only here till the webhook works. it might stay after, but we will see.

#[post("/runners/{runner}/queue")]
/// Queues a job for the specified runner.
async fn queue_job_run(
    ws: web::Data<Spire>,
    _req: HttpRequest,
    data: web::Json<RequestJson>,
    runner: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let repo_info = data.into_inner();
    queue_job(
        ws.into_inner(),
        repo_info.name,
        repo_info.owner,
        "manual".to_string(),
        runner.into_inner(),
        true,
    )
    .await
    .unwrap();
    Ok(HttpResponse::Ok().finish())
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct _GitUser {
//     pub name: String,
//     pub email: String,
//     pub username: String
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Commit {
//     pub id: String,
//     pub tree_id: String,
//     pub distinct: bool,
//     pub message: String,
//     pub timestamp: String,
//     pub url: String,
//     pub author: _GitUser,
//     pub committer: _GitUser,
//     pub added: Vec<String>,
//     pub removed: Vec<String>,
//     pub modified: Vec<String>
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct _Repository {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PushEvent {
    pub repository: _Repository,
    // pub before: String,
    // pub after: String,
    // pub pusher: GitUser,
    // pub organization: Option<Organization>,
    // pub sender: User,
    // pub created: bool,
    // pub deleted: bool,
    // pub forced: bool,
    // pub base_ref: Option<String>,
    // pub compare: String,
    // pub commits: Vec<Commit>,
    // pub head_commit: Commit
}

#[post("webhook")]
async fn github_webhook(
    app: web::Data<Spire>,
    _req: HttpRequest,
    data: web::Json<serde_json::Value>,
) -> Result<HttpResponse, Error> {
    let payload = data.into_inner();
    if let Ok(payload) = serde_json::from_value::<PushEvent>(payload) {
        let Ok(repo) = sqlx::query_as::<_, spiar::models::Repos>(r#"SELECT * FROM repos WHERE gh_id = ($1)"#).bind(&payload.repository.id).fetch_one(&app.database.0).await else {
            return Ok(HttpResponse::BadRequest().finish())
        };

        // TODO: Cache
        let Ok(runners) = sqlx::query_as::<_, spiar::models::Runners>(r#"SELECT * FROM runners"#).fetch_all(&app.database.0).await else {
            return Ok(HttpResponse::BadRequest().finish())
        };

        for runner in runners {
            let Ok(jobs) = sqlx::query_as::<_, spiar::models::Job>(r#"SELECT * FROM job WHERE assigned_runner = $1 AND status IN (0,1)"#).bind(&runner.name).fetch_all(&app.database.0).await else {
                return Ok(HttpResponse::InternalServerError().finish());
            };

            if jobs.len() <= 0 {
                if queue_job(
                    app.into_inner(),
                    repo.name,
                    repo.owner,
                    "webhook".to_string(),
                    runner.name,
                    false,
                )
                .await
                .is_err()
                {
                    return Ok(HttpResponse::InternalServerError().finish());
                }
                break;
            }
        }
        return Ok(HttpResponse::Ok().finish());
    }
    Ok(HttpResponse::BadRequest().finish())
}

#[actix_web::main]
async fn main() -> Result<()> {
    let _ = PLUGINS;
    let host_and_port = match CONFIG.to_owned().server {
        Some(server) => (server.host, server.port),
        None => ("0.0.0.0".to_string(), 8080),
    };

    let database = Database::new(CONFIG.to_owned().database).await?;
    database.migrate().await?;

    let app = web::Data::new(Spire {
        connections: Connections {
            connected_runners: HashMap::new(),
        }
        .start(),
        database: Arc::new(database),
    });

    HttpServer::new(move || {
        App::new()
            .service(
                scope("api")
                    .service(manage_new_install)
                    .service(create_ws_session)
                    .service(queue_job_run),
            )
            .service(github_webhook)
            .service(get_repos)
            .service(get_repo_jobs)
            .service(get_job_logs)
            .app_data(app.clone())
    })
    .bind(host_and_port)?
    .run()
    .await
    .unwrap();

    Ok(())
}