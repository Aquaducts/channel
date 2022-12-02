use std::collections::HashMap;

use actix::{Actor, Context, Recipient};

pub mod database;
pub mod messages;
pub mod models;
pub mod socket;

pub struct Spire {
    pub connected_runners: HashMap<String, Recipient<messages::BaseMessage>>,
}

impl Spire {
    fn send_message(&self, message: &str, id_to: String) {
        if let Some(socket_recipient) = self.connected_runners.get(&id_to) {
            socket_recipient.do_send(messages::BaseMessage(message.to_owned()));
        } else {
            println!("attempting to send message but couldn't find user id.");
        }
    }
}

impl Actor for Spire {
    type Context = Context<Self>;
}
