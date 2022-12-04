use crate::{
    database::Database,
    messages::{BaseMessage, Connect, Disconnect},
    models::Repos,
    Spire,
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Running,
    StreamHandler, WrapFuture,
};
use actix_web_actors::ws;
use async_trait::async_trait;
use std::sync::Arc;

pub struct SocketSession {
    pub app: Addr<Spire>,
    pub runner: String,
    pub database: Arc<Database>,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(message)) => {
                let message = message.to_string();
                if let Ok(message) =
                    serde_json::from_str::<bamboo_common::websocket::Messages>(&message)
                {
                    match message {
                        bamboo_common::websocket::Messages::GetJobRepo { job, repo } => {
                            println!("JOB WANTS REPO!");
                            let database = self.database.clone();
                            let recipient = ctx.address().recipient();
                            let fut = async move {
                                let repo = sqlx::query_as::<_, Repos>(
                                    r#"SELECT * FROM repos WHERE id = $1"#,
                                )
                                .bind(&repo)
                                .fetch_one(&database.0)
                                .await
                                .unwrap();

                                recipient
                                    .do_send(BaseMessage(serde_json::to_string(&repo).unwrap()));
                                return;
                            };

                            fut.into_actor(self).spawn(ctx);
                        }
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
