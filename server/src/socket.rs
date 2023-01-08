use crate::{
    config::{CLIENT_TIMEOUT, HEARTBEAT_INTERVAL},
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
use common::{
    events::Identify,
    websocket::{Messages, WebsocketMessage},
    RepoConfig, Spurs,
};
use std::{
    any::TypeId,
    sync::Arc,
    time::{Duration, Instant},
};

/*

ws.connections
    .send(NewAndImprovedMessage(
        String::from("runner1"),
        common::websocket::WebsocketMessage {
            op: common::websocket::OpCodes::Hello,
            event: Some(Box::new(common::websocket::Hello {
                fake: String::from("HI"),
            })),
        },
    ))
    .await
    .unwrap();*/
pub struct SocketSession {
    pub app: Addr<Connections>,
    pub runner: String,
    pub identified: bool,
    pub database: Arc<Database>,
    pub heartbeat: Instant,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(message)) => {
                let _message = message.to_string();
                if let Ok(message) = serde_json::from_str::<WebsocketMessage>(&_message) {
                    println!("{:?}", message);
                    match message.op {
                        common::websocket::OpCodes::EventCreate => todo!(),
                        common::websocket::OpCodes::Hello => todo!(),
                        common::websocket::OpCodes::Identify => {
                            let database = self.database.clone();
                            let fut = async move {
                                let Some(d) = message.event else {
                                    return;
                                };

                                let d_any = d.as_any();
                                if d_any.type_id() == TypeId::of::<Identify>() {
                                    println!("GOOD?");
                                }
                                return;
                            };
                            fut.into_actor(self).spawn(ctx);
                        }
                        common::websocket::OpCodes::HeartBeat => todo!(),
                        common::websocket::OpCodes::HeartBeatAck => todo!(),
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
        // notify chat server
        self.app.do_send(Disconnect {
            runner: self.runner.clone(),
        });
        Running::Stop
    }

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("STARTED");
        self.hb(ctx);

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

impl SocketSession {
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        let identified = self.identified.clone();
        ctx.run_interval(Duration::from_secs(HEARTBEAT_INTERVAL), move |act, ctx| {
            if !identified {
                println!("Hasnt identified yet.");
            }
            if Instant::now().duration_since(act.heartbeat) > Duration::from_secs(CLIENT_TIMEOUT) {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify chat server
                //act.addr.do_send(server::Disconnect { id: act.id });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}
