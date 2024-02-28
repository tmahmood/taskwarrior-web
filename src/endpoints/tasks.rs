use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, trace};
use std::process::Command;
use std::fs;
use std::fs::File;
use std::io::Error;
use std::str::FromStr;

pub const TASK_DATA_FILE: &str = "data.json";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub description: String,
    pub end: Option<String>,
    pub project: Option<String>,
    pub entry: String,
    pub modified: String,
    pub status: String,
    pub uuid: String,
    pub urgency: f64,
    pub wait: Option<String>,
    pub tags: Option<Vec<String>>,
    pub priority: Option<String>
}

pub fn fetch_task_from_cmd(params: Vec<&str>) -> Result<(), anyhow::Error>{
    let data_file = PathBuf::from_str(TASK_DATA_FILE).unwrap();
    let mut task = Command::new("task");
    if params.len() > 0 {
        for param in params.iter() {
            task.arg(param);
        }
    }
    trace!("{:?}", params);
    match task
        .arg("export").output()
        .and_then(|v| {
            // write the output to file,
            fs::write(&data_file, v.stdout)
        }) {
        Ok(_) => Ok(()),
        Err(_) => anyhow::bail!("Failed to read tasks")
    }
}

// what would happen
pub fn list_tasks(params: Vec<&str>) -> Result<Vec<Task>, anyhow::Error> {
    fetch_task_from_cmd(params)?;
    let data_file = PathBuf::from_str(TASK_DATA_FILE).unwrap();
    let content = fs::read_to_string(&data_file)?;
    match serde_json::from_str(&content) {
        Ok(s) => Ok(s),
        Err(e) => anyhow::bail!(e.to_string())
    }
}

pub fn update_tasks(tasks: &mut Vec<Task>) -> Result<(), anyhow::Error> {

    Ok(())
}
