use std::any::Any;

use chrono::Duration;

use serde::{Deserialize, Serialize};

use crate::websocket::WebsocketEvent;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello {
    pub heartbeat: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identify {
    pub name: String,
    pub password: String,
}

#[typetag::serde]
impl WebsocketEvent for Hello {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
#[typetag::serde]
impl WebsocketEvent for Identify {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
