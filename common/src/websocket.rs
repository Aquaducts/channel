use crate::{Job, RepoConfig, Repos};
use serde::{Deserialize, Serialize};
use std::{
    any::{Any, TypeId},
    boxed::Box,
    fmt::Debug,
};

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "op")]
pub enum OpCodes {
    EventCreate = 0,
    Hello = 1,
    Identify = 2,
    HeartBeat = 3,
    HeartBeatAck = 4,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WebsocketMessage {
    pub op: OpCodes,
    pub event: Option<Box<dyn WebsocketEvent + Send + Sync + Sized>>,
}

#[typetag::serde]
pub trait WebsocketEvent: erased_serde::Serialize + Debug + Send + Sync + Sized + Clone {
    fn as_any(&self) -> &dyn Any;
}

impl WebsocketMessage {
    pub fn downcast_event<T: 'static>(&self) -> Option<&T> {
        let Some(event) = &self.event else {
            return None;
        };
        let event_any = event.as_any();
        if event_any.type_id() != TypeId::of::<T>() {
            // The event type did not match the type we wanted
            return None;
        }
        event_any.downcast_ref::<T>()
    }
}

#[repr(C)]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Messages {
    CreateJobRun {
        job: Job,
    },
    GetJobRepo {
        job: i64,
        repo: i64,
    },
    GetRepoConfig {
        repo: i64,
    },
    RepoConfig(RepoConfig),
    Repo(Repos),
    CreateJobLog {
        job: i64,
        status: i64,
        step: String,
        pipe: String,
        output: String,
    },
    UpdateJobStatus {
        job: i64,
        status: i64,
    },
}
