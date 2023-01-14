use crate::{config::HEARTBEAT_INTERVAL, models::Job, socket::SocketSession, Connections};
use actix::{Context, Handler, Message, Recipient};
use common::websocket::{Messages, WebsocketMessage};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use std::boxed::Box;

#[derive(Message)]
#[rtype(result = "Option<()>")]
pub struct NewAndImprovedMessage(pub String, pub WebsocketMessage);

impl Handler<NewAndImprovedMessage> for Connections {
    type Result = Option<()>;

    fn handle(&mut self, msg: NewAndImprovedMessage, ctx: &mut Context<Self>) -> Self::Result {
        println!("{:?}", msg.1);
        self.send_message(&to_string(&msg.1).unwrap(), msg.0);
        Some(())
    }
}

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

impl Handler<Disconnect> for Connections {
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

impl Handler<Connect> for Connections {
    type Result = Option<()>;

    fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
        let runner = msg.runner.clone();
        // Don't allow "runners" (maybe a person who is bad) to connect when another runner is already connected.
        if self.connected_runners.get(&runner).is_some() {
            return None;
        }

        self.connected_runners.insert(msg.runner.clone(), msg.addr);
        self.send_message(
            &to_string(&common::websocket::WebsocketMessage {
                op: common::websocket::OpCodes::Hello,
                event: Some(Box::new(common::events::Hello {
                    heartbeat: HEARTBEAT_INTERVAL,
                })),
            })
            .unwrap(),
            msg.runner,
        );
        Some(())
    }
}

#[derive(Message, Deserialize, Serialize, Clone)]
#[rtype(result = "()")]
pub struct JobRequest {
    pub runner: String,
    pub job: Job,
}

impl Handler<JobRequest> for Connections {
    type Result = ();

    fn handle(&mut self, job_request: JobRequest, _ctx: &mut Self::Context) {
        self.send_message(
            &to_string(&Messages::CreateJobRun {
                job: job_request.job,
            })
            .unwrap(),
            job_request.runner,
        );
    }
}
