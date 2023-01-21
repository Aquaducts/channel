use actix::Actor;

use actix_cors::Cors;
use actix_web::{
    get, post,
    web::{self},
    App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_actors::ws;
use anyhow::Result;

use serde::{Deserialize, Serialize};
use spiar::{
    api::{
        github::manage_new_install,
        jobs::{get_specific_job, job_search},
    },
    config::CONFIG,
    database::Database,
    errors::Error,
    messages::{JobRequest, NewAndImprovedMessage},
    models::{Job, Repos, Runners},
    socket::SocketSession,
    Connections, Spire,
};
use sqlx::FromRow;
use std::{collections::HashMap, fs::read_to_string, pin::Pin, sync::Arc, time::Instant};

#[get("/ws")]
async fn create_ws_session(
    ws: web::Data<Spire>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<impl Responder, Error> {
    let new_connection = SocketSession {
        app: Pin::new(&ws.connections).get_ref().clone(),
        runner: None,
        database: ws.database.clone(),
        heartbeat: Instant::now(),
        identified: Arc::new(std::sync::Mutex::new(false)),
    };
    // fix
    let resp = ws::start(new_connection, &req, stream).unwrap();
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
            .send(NewAndImprovedMessage(
                runner,
                common::websocket::WebsocketMessage {
                    op: common::websocket::OpCodes::EventCreate,
                    event: Some(Box::new(common::events::CreateJobRun { job: new_job })),
                },
            ))
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
        ws.connections
            .send(NewAndImprovedMessage(
                runner,
                common::websocket::WebsocketMessage {
                    op: common::websocket::OpCodes::EventCreate,
                    event: Some(Box::new(common::events::CreateJobRun { job: new_job })),
                },
            ))
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
) -> Result<impl Responder, Error> {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct _Repository {
    pub id: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PushEvent {
    pub repository: _Repository,
}

#[post("webhook")]
async fn github_webhook(
    app: web::Data<Spire>,
    _req: HttpRequest,
    data: web::Json<serde_json::Value>,
) -> Result<impl Responder, Error> {
    let payload = data.into_inner();
    if let Ok(payload) = serde_json::from_value::<PushEvent>(payload) {
        let Ok(repo) = sqlx::query_as::<_, spiar::models::Repos>(r#"SELECT * FROM repos WHERE gh_id = ($1)"#).bind(payload.repository.id).fetch_one(&app.database.0).await else {
            return Err(Error::bad_request("Requested repo is not configured."));
        };

        // TODO: Cache runners
        let Ok(runners) = sqlx::query_as::<_, spiar::models::Runners>(r#"SELECT * FROM runners"#).fetch_all(&app.database.0).await else {
            return Err(Error::internal_server_error("Failed to get the runners"));
        };

        for runner in runners {
            let Ok(jobs) = sqlx::query_as::<_, spiar::models::Job>(r#"SELECT * FROM job WHERE assigned_runner = $1 AND status IN (0,1)"#).bind(&runner.name).fetch_all(&app.database.0).await else {
                return Err(Error::internal_server_error("Failed to get a job."));
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
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .pretty()
        .init();

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
            .wrap(Cors::permissive())
            .service(
                web::scopescope("api")
                    .service(manage_new_install)
                    .service(create_ws_session)
                    .service(queue_job_run),
            )
            .service(github_webhook)
            .service(job_search)
            .service(get_specific_job)
            .app_data(app.clone())
    })
    .bind(host_and_port)?
    .run()
    .await
    .unwrap();

    Ok(())
}
