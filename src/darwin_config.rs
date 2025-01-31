use std::{collections::HashMap, fs::{File, OpenOptions}, io::{ErrorKind, Write}};
use std::io::{Result, Error};

use serde::{Serialize, Deserialize};

use crate::config::darwin_config;
use strum::EnumIter;

#[derive(Serialize, Deserialize, Debug, Clone, clap::ValueEnum, EnumIter)]
pub enum ProjectType {
    None,
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
    let file = OpenOptions::new().read(true).create_new(false).open(darwin_config())
        .map_err(|e|Error::new(ErrorKind::Other, format!("Failed to open darwin config file for reading: {}", e)))?;
    serde_json::from_reader(file).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to parse darwin config: {}", e)))
}

pub fn write_config(config: DarwinConfig) -> Result<()> {
    let mut file = File::create(darwin_config()).map_err(|e|Error::new(ErrorKind::Other, format!("Failed to create darwin_config file: {}", e)))?;
    serde_json::to_writer_pretty(&file, &config)?;
    file.flush()?;
    Ok(())
}

pub fn list_tests() -> Vec<String> {
    read_config().unwrap().tests
}