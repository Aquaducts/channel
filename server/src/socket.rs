use crate::{
    database::Database,
    messages::{BaseMessage, Connect, Disconnect, JobRequest},
    models::{Job, Repos},
    Connections,
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Running,
    StreamHandler, WrapFuture,
};
use actix_web_actors::ws;
use common::{websocket::Messages, RepoConfig, Spurs};
use std::sync::Arc;

pub struct SocketSession {
    pub app: Addr<Connections>,
    pub runner: String,
    pub database: Arc<Database>,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(message)) => {
                let _message = message.to_string();
                if let Ok(message) = serde_json::from_str::<common::websocket::Messages>(&_message)
                {
                    match message {
                        common::websocket::Messages::CreateJobLog {
                            job,
                            status,
                            step,
                            output,
                            pipe,
                        } => {
                            let database = self.database.clone();
                            let fut = async move {
                                println!("{:?}", sqlx::query::<_>(
                                    r#"INSERT INTO job_logs(job, step, status, output, pipe) VALUES($1,$2,$3,$4,$5)"#,
                                )
                                .bind(job)
                                .bind(&step)
                                .bind(status)
                                .bind(&output)
                                .bind(&pipe)
                                .execute(&database.0)
                                .await.unwrap());
                            };

                            fut.into_actor(self).spawn(ctx);
                        }
                        common::websocket::Messages::GetJobRepo { job: _, repo } => {
                            let database = self.database.clone();
                            let recipient = ctx.address().recipient();
                            let fut = async move {
                                let repo = sqlx::query_as::<_, Repos>(
                                    r#"SELECT * FROM repos WHERE id = $1"#,
                                )
                                .bind(repo)
                                .fetch_one(&database.0)
                                .await
                                .unwrap();

                                recipient.do_send(BaseMessage(
                                    serde_json::to_string(&Messages::Repo(repo)).unwrap(),
                                ));
                            };

                            fut.into_actor(self).spawn(ctx);
                        }
                        common::websocket::Messages::GetRepoConfig { repo } => {
                            let database = self.database.clone();
                            let recipient = ctx.address().recipient();
                            let repo_id = repo;
                            let fut = async move {
                                let _repo = sqlx::query_as::<_, Repos>(
                                    r#"SELECT * FROM repos WHERE id = $1"#,
                                )
                                .bind(repo_id)
                                .fetch_one(&database.0)
                                .await
                                .unwrap();

                                let spurs = sqlx::query_as::<_, Spurs>(
                                    r#"SELECT * FROM spurs WHERE owned_by = $1"#,
                                )
                                .bind(repo_id)
                                .fetch_all(&database.0)
                                .await
                                .unwrap();

                                recipient.do_send(BaseMessage(
                                    serde_json::to_string(&Messages::RepoConfig(RepoConfig {
                                        spurs,
                                    }))
                                    .unwrap(),
                                ));
                            };

                            fut.into_actor(self).spawn(ctx);
                        }
                        common::websocket::Messages::UpdateJobStatus { job, status } => {
                            let database = self.database.clone();
                            let recipient = ctx.address().recipient();
                            let fut = async move {
                                let mut query_base = String::from("SET \"status\" = $1");
                                if status == 3 {
                                    query_base.push_str(",\"end\" = NOW() AT TIME ZONE 'utc'")
                                }

                                let job = sqlx::query_as::<_, Job>(&format!(
                                    r#"UPDATE job {query_base} WHERE id = $2 RETURNING *"#
                                ))
                                .bind(status)
                                .bind(job)
                                .fetch_one(&database.0)
                                .await
                                .map_err(|e| println!("{:?}", e))
                                .unwrap();

                                // Check for the next job when this one is complete
                                if status == 3 && job.status == 3 {
                                    let next_job = sqlx::query_as::<_, Job>(
                                        r#"SELECT * FROM job WHERE status = 0 ORDER BY id ASC"#,
                                    )
                                    .fetch_one(&database.0)
                                    .await;

                                    if let Ok(next_job) = next_job {
                                        recipient.do_send(BaseMessage(
                                            serde_json::to_string(&JobRequest {
                                                runner: next_job.assigned_runner.to_string(),
                                                job: next_job,
                                            })
                                            .unwrap(),
                                        ));
                                    }
                                }
                            };

                            fut.into_actor(self).spawn(ctx);
                        }
                        _ => {}
                    }
                }
            }
            _ => (),
        }
    }
}

impl Actor for SocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.app.do_send(Disconnect {
            runner: self.runner.clone(),
        });
        Running::Stop
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("STARTED");

        let addr = ctx.address();
        let runner = self.runner.to_owned();

        self.app
            .send(Connect {
                addr: addr.recipient(),
                runner,
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => {
                        if _res.is_none() {
                            ctx.close(None);
                            ctx.stop()
                        }
                    }
                    _ => {
                        ctx.close(None);
                        ctx.stop()
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}
