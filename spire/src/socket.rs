use crate::{
    messages::{Connect, Disconnect},
    Spire,
};
use actix::{
    fut, Actor, ActorContext, ActorFutureExt, Addr, AsyncContext, ContextFutureSpawner, Running,
    StreamHandler, WrapFuture,
};
use actix_web_actors::ws;

pub struct SocketSession {
    pub app: Addr<Spire>,
    pub runner: String,
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
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
