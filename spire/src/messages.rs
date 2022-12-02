use crate::{socket::SocketSession, Spire};
use actix::{Context, Handler, Message, Recipient};
use serde::{Deserialize, Serialize};
use serde_json::to_string;

#[derive(Message)]
#[rtype(result = "()")]
pub struct BaseMessage(pub String);

impl Handler<BaseMessage> for SocketSession {
    type Result = ();

    fn handle(&mut self, msg: BaseMessage, ctx: &mut Self::Context) {
        ctx.text(msg.0);
    }
}

#[derive(Message)]
#[rtype(result = "Option<()>")]
pub struct Disconnect {
    pub runner: String,
}

impl Handler<Disconnect> for Spire {
    type Result = Option<()>;

    fn handle(&mut self, msg: Disconnect, _ctx: &mut Context<Self>) -> Self::Result {
        let runner = msg.runner;
        // Don't allow "runners" (maybe a person who is bad) to connect when another runner is already connected.
        self.connected_runners.remove(&runner);
        Some(())
    }
}

#[derive(Message)]
#[rtype(result = "Option<()>")]
pub struct Connect {
    pub addr: Recipient<BaseMessage>,
    pub runner: String,
}

impl Handler<Connect> for Spire {
    type Result = Option<()>;

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        let runner = msg.runner.clone();
        // Don't allow "runners" (maybe a person who is bad) to connect when another runner is already connected.
        if self.connected_runners.get(&runner).is_some() {
            return None;
        }

        self.connected_runners.insert(msg.runner.clone(), msg.addr);
        self.send_message(&format!("Hi builder: {}", msg.runner), msg.runner);
        Some(())
    }
}

#[derive(Message, Deserialize, Serialize, Clone)]
#[rtype(result = "()")]
pub struct JobRequest {
    pub runner: String,
    /// Uhm full url? or just like owner/name? idk for now ima just use the entire repo url.
    pub repo: String,
}

impl Handler<JobRequest> for Spire {
    type Result = ();

    fn handle(&mut self, job_request: JobRequest, _ctx: &mut Self::Context) {
        self.send_message(&to_string(&job_request).unwrap(), job_request.runner);
    }
}
