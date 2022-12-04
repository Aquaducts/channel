use actix::{Actor, Addr};
use actix_web::{get, post, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use spiar::{
    config::Config,
    database::Database,
    messages::JobRequest,
    models::{Job, Runners},
    socket::SocketSession,
    Spire,
};
use sqlx::FromRow;
use std::{collections::HashMap, fs::read_to_string, sync::Arc};

pub struct RealApp {
    pub database: Arc<Database>,
}

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

#[get("/ws")]
async fn create_ws_session(
    ws: web::Data<Addr<Spire>>,
    app: web::Data<RealApp>,
    config: web::Data<Config>,
    params: web::Query<ConnectRequest>,
    req: HttpRequest,
    stream: web::Payload,
) -> Result<HttpResponse, Error> {
    let (name, password) = (params.0.name, params.0.password);
    let Ok(possible_runner) = sqlx::query_as::<_, Runners>(r#"SELECT * FROM runners WHERE name = $1"#)
        .bind(&name)
        .fetch_one(&app.database.0)
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
        app: ws.get_ref().clone(),
        runner: name,
        database: app.database.clone(),
    };
    let resp = ws::start(new_connection, &req, stream)?;
    Ok(resp)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestJson {
    pub repo: String,
}

#[derive(Debug, FromRow, Clone)]
pub struct JobCount(i64);

pub async fn queue_job(
    app: Arc<RealApp>,
    ws: Arc<Addr<Spire>>,
    _request: RequestJson,
    runner: String,
) -> Result<()> {
    // do some repo magic when github notifications start to work.
    let new_job = sqlx::query_as::<_, Job>(
        r#"INSERT INTO job(assigned_runner, repo) VALUES($1, 1) RETURNING *"#,
    )
    .bind(&runner)
    .fetch_one(&app.database.0)
    .await?;

    let all_possible_queued_jobs = sqlx::query_as::<_, JobCount>(
        r#"SELECT count(*) FROM job WHERE status = 0 AND assigned_runner = $1"#,
    )
    .bind(&runner)
    .fetch_one(&app.database.0)
    .await?;

    if all_possible_queued_jobs.0 <= 1 {
        println!("sent");
        ws.send(JobRequest {
            runner,
            job: new_job,
        })
        .await?;
    }

    Ok(())
}

#[post("/runners/{runner}/queue")]
/// Queues a job for the specified runner.
async fn queue_job_run(
    ws: web::Data<Addr<Spire>>,
    _app: web::Data<RealApp>,
    _config: web::Data<Config>,
    _req: HttpRequest,
    data: web::Json<RequestJson>,
    runner: web::Path<String>,
) -> Result<HttpResponse, Error> {
    queue_job(
        _app.into_inner(),
        ws.into_inner(),
        data.into_inner(),
        runner.into_inner(),
    )
    .await
    .unwrap();
    Ok(HttpResponse::Ok().finish())
}

#[actix_web::main]
async fn main() -> Result<()> {
    let config = std::fs::read_to_string("./spire/Config.toml")?;
    let config = toml::from_str::<Config>(&config)?;

    let host_and_port = match config.clone().server {
        Some(server) => (server.host, server.port),
        None => ("0.0.0.0".to_string(), 8080),
    };

    let database = Database::new(config.database.to_string()).await?;
    database.migrate().await?;

    let websocket = web::Data::new(
        Spire {
            connected_runners: HashMap::new(),
            config: config.clone(),
        }
        .start(),
    );
    let config_data = web::Data::new(config.clone());
    let app = web::Data::new(RealApp {
        database: Arc::new(database),
    });
    HttpServer::new(move || {
        App::new()
            .service(create_ws_session)
            .service(queue_job_run)
            .app_data(websocket.clone())
            .app_data(app.clone())
            .app_data(config_data.clone())
    })
    .bind(host_and_port)?
    .run()
    .await
    .unwrap();

    Ok(())
}
