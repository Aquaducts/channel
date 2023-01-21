use crate::{
    config::{CLIENT_TIMEOUT, HEARTBEAT_INTERVAL},
    database::Database,
    messages::{self, BaseMessage, Connect, Disconnect, JobRequest},
    models::{Job, Repos},
    Connections,
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Running,
    StreamHandler, WrapFuture,
};
use actix_web_actors::ws;
use common::{
    events::{Identify, RequestRepoConfig},
    websocket::{Messages, WebsocketMessage},
    RepoConfig, Runners, Spurs,
};
use serde::{Deserialize, Serialize};
use std::{
    any::TypeId,
    sync::Arc,
    time::{Duration, Instant},
};

use std::sync::Mutex;

pub struct SocketSession {
    pub app: Addr<Connections>,
    pub runner: Option<Arc<Mutex<String>>>,
    pub identified: Arc<Mutex<bool>>,
    pub database: Arc<Database>,
    pub heartbeat: Instant,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(message)) => {
                let _message = message.to_string();
                if let Ok(message) = serde_json::from_str::<WebsocketMessage>(&_message) {
                    match message.op {
                        common::websocket::OpCodes::EventCreate => {
                            let database = self.database.clone();
                            let recipient = ctx.address().recipient();
                            let fut = async move {
                                let Some(d) = message.event else {
                                    return;
                                };

                                let d_any = d.as_any();
                                if d_any.type_id() == TypeId::of::<RequestRepoConfig>() {
                                    let Some(request_repo_config) = d_any.downcast_ref::<RequestRepoConfig>() else {
                                        return;
                                    };

                                    let Ok(spurs) = sqlx::query_as::<_, Spurs>(r#"SELECT * FROM spurs WHERE owned_by = ($1)"#).bind(&request_repo_config.repo).fetch_all(&database.0).await else {
                                        return;
                                    };
                                    let Ok(repo_config) = sqlx::query_as::<_, Repos>(r#"SELECT * FROM repos WHERE id = ($1)"#).bind(&request_repo_config.repo).fetch_one(&database.0).await else {
                                        return;
                                    };

                                    recipient.do_send(BaseMessage(
                                        serde_json::to_string(
                                            &common::websocket::WebsocketMessage {
                                                op: common::websocket::OpCodes::EventCreate,
                                                event: Some(Box::new(common::events::RepoConfig {
                                                    repo: repo_config,
                                                    spurs,
                                                })),
                                            },
                                        )
                                        .unwrap(),
                                    ))
                                }
                                return;
                            };
                            fut.into_actor(self).spawn(ctx);
                        }
                        common::websocket::OpCodes::Identify => {
                            let database = self.database.clone();
                            let app = self.app.clone();
                            let recipient = ctx.address().recipient();
                            let identified = self.identified.clone();
                            let fut = async move {
                                let Some(d) = message.event else {
                                    return;
                                };

                                let d_any = d.as_any();
                                if d_any.type_id() == TypeId::of::<Identify>() {
                                    let Some(data) = d_any.downcast_ref::<Identify>() else {
                                        return;
                                    };

                                    let Ok(possible_runner) = sqlx::query_as::<_, Runners>(r#"SELECT * FROM runners WHERE name = $1"#)
                                    .bind(&data.name)
                                    .fetch_one(&database.0)
                                    .await else {
                                        return;
                                    };

                                    let Ok(config_file) = std::fs::read_to_string(format!("{}/Config.toml", possible_runner.local_path)) else {
                                        return;
                                    };

                                    // TODO: move this to common
                                    #[derive(Serialize, Deserialize, Debug, Clone)]
                                    pub struct RunnerConfigFile {
                                        pub name: String,
                                        pub password: Option<String>,
                                    }

                                    let Ok(deserialized) = toml::from_str::<RunnerConfigFile>(&config_file) else {
                                        return;
                                    };
                                    if let Some(runner_pass) = deserialized.password {
                                        if data.password != runner_pass {
                                            return;
                                        }
                                    }

                                    let identified = identified.clone();
                                    let mut identified = identified.lock().unwrap();
                                    *identified = true;
                                    app.send(Connect {
                                        addr: recipient,
                                        runner: deserialized.name,
                                    })
                                    .await
                                    .unwrap();
                                }
                                return;
                            };

                            self.heartbeat = Instant::now();
                            fut.into_actor(self).spawn(ctx);
                        }
                        common::websocket::OpCodes::HeartBeat => todo!(),
                        common::websocket::OpCodes::HeartBeatAck => {
                            // Do more later or smth
                            self.heartbeat = Instant::now();
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

impl Actor for SocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        if let Some(runner) = self.runner.clone() {
            let runner_name = runner.lock().unwrap();
            self.app.do_send(Disconnect {
                runner: (*runner_name.clone()).to_string(),
            });
        }
        Running::Stop
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("STARTED");
        self.hb(ctx);
        let addr = ctx.address();

        let recipient = addr.recipient();
        recipient.do_send(messages::BaseMessage(
            serde_json::to_string(&common::websocket::WebsocketMessage {
                op: common::websocket::OpCodes::Hello,
                event: Some(Box::new(common::events::Hello {
                    heartbeat: HEARTBEAT_INTERVAL,
                })),
            })
            .unwrap(),
        ));
    }
}

impl SocketSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let identified = self.identified.clone();
        ctx.run_interval(Duration::from_secs(HEARTBEAT_INTERVAL), move |act, ctx| {
            let identified = identified.clone();
            let identified = identified.lock().unwrap();
            if !*identified {
                println!("Websocket Client not identified!");
                ctx.stop();
                return;
            }

            if Instant::now().duration_since(act.heartbeat) > Duration::from_secs(CLIENT_TIMEOUT) {
                println!("Websocket Client heartbeat failed, disconnecting!");
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}
