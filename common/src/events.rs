use std::any::Any;

use chrono::Duration;

use serde::{Deserialize, Serialize};

use crate::{websocket::WebsocketEvent, Job, Repos, Spurs};

macro_rules! impl_websocket_event {
    ($name:ident) => {
        #[typetag::serde]
        impl WebsocketEvent for $name {
            fn as_any(&self) -> &dyn Any {
                self
            }
        }
    };
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Hello {
    pub heartbeat: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identify {
    pub name: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateJobRun {
    pub job: Job,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestRepoConfig {
    pub repo: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepoConfig {
    pub repo: Repos,
    pub spurs: Vec<Spurs>,
}

impl_websocket_event!(Hello);
impl_websocket_event!(Identify);
impl_websocket_event!(CreateJobRun);
impl_websocket_event!(RequestRepoConfig);
impl_websocket_event!(RepoConfig);
