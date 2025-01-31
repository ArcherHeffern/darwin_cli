use std::{collections::HashMap, fs::{File, OpenOptions}, io::ErrorKind};
use std::io::{Result, Error};

use serde::{Serialize, Deserialize};

use crate::config::darwin_config;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ProjectType {
    MavenSurefire,
    Go,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DarwinConfig {
    pub version: String,
    pub project_type: ProjectType,
    pub tests: Vec<String>,
    pub tests_run: Vec<String>,
    pub extraction_errors: HashMap<String, String>,
}

pub fn read_config() -> Result<DarwinConfig> {
    let file = OpenOptions::new().read(true).create_new(false).open(darwin_config())?;
    serde_json::from_reader(file).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to open darwin config: {}", e)))
}
pub fn list_tests() -> Vec<String> {
    read_config().unwrap().tests
}