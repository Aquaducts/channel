use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pipe {
    pub steps: Vec<Step>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Step {
    pub name: Option<String>,
    pub run: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OverallFile {
    pub pipes: HashMap<String, Pipe>,
}
