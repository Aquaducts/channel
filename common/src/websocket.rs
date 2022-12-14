use serde::{Deserialize, Serialize};

use crate::{Job, Repos};

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
    RepoConfig(String),
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
