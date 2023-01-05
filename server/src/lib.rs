use std::{collections::HashMap, sync::Arc};

use actix::{Actor, Addr, Context, Recipient};

pub mod api;
pub mod config;
pub use common::database;
pub mod github;
pub mod messages;
pub mod models;
pub mod plugins;
pub mod socket;

pub struct Connections {
    pub connected_runners: HashMap<String, Recipient<messages::BaseMessage>>,
}

impl Connections {
    fn send_message(&self, message: &str, id_to: String) {
        if let Some(socket_recipient) = self.connected_runners.get(&id_to) {
            socket_recipient.do_send(messages::BaseMessage(message.to_owned()));
        } else {
            println!("attempting to send message but couldn't find user id.");
        }
    }
}

pub struct Spire {
    pub connections: Addr<Connections>,
    pub database: Arc<database::Database>,
}

impl Actor for Connections {
    type Context = Context<Self>;
}
