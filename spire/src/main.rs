use actix::{Actor, Addr};
use actix_web::{get, post, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use anyhow::Result;
use serde::{Deserialize, Serialize};

use spiar::{
    config::Config, database::Database, messages::JobRequest, models::Runners,
    socket::SocketSession, Spire,
};
use std::{collections::HashMap, fs::read_to_string};

pub struct RealApp {
    pub database: Database,
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
    };
    let resp = ws::start(new_connection, &req, stream)?;
    Ok(resp)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RequestJson {
    pub repo: String,
}

#[post("/runners/{runner}/run")]
/// Queues a job for the specified runner.
async fn queue_job_run(
    ws: web::Data<Addr<Spire>>,
    _app: web::Data<RealApp>,
    config: web::Data<Config>,
    _req: HttpRequest,
    data: web::Json<RequestJson>,
    runner: web::Path<String>,
) -> Result<HttpResponse, Error> {
    let result = ws
        .send(JobRequest {
            runner: runner.into_inner(),
            repo: data.0.repo,
        })
        .await;
    if result.is_err() {
        // Make custom errors for all these message types.
        return Ok(HttpResponse::InternalServerError().finish());
    }
    Ok(HttpResponse::Ok().finish())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = std::fs::read_to_string("./Config.toml")?;
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
    let app = web::Data::new(RealApp { database });
    let server = HttpServer::new(move || {
        App::new()
            .service(create_ws_session)
            .service(queue_job_run)
            .app_data(websocket.clone())
            .app_data(app.clone())
            .app_data(config.clone())
    });

    server.bind(host_and_port)?.run().await.unwrap();

    Ok(())
}
