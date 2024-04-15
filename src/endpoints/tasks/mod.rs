use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, error, info, trace};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::fs;
use std::fs::File;
use std::io::Error;
use std::str::FromStr;
use anyhow::anyhow;
use indexmap::IndexMap;
use serde_json::Value;
use tokio::io::split;

pub mod task_query_builder;

use task_query_builder::TaskQuery;
use crate::{Params, TaskUpdateStatus};

pub const TASK_DATA_FILE: &str = "data.json";
pub const TASK_DATA_FILE_EDIT: &str = "data_edit.json";
pub const TASK_OUTPUT_FILE: &str = "output.out";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Annotation {
    entry: String,
    description: String
}


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due: Option<String>,
    pub modified: Option<String>,
    pub status: Option<String>,
    pub uuid: String,
    pub urgency: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recur: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imask: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename="UDA")]
    pub uda: Option<HashMap<String, Value>>
}

pub fn fetch_task_from_cmd(
    task_query: &TaskQuery
) -> Result<PathBuf, anyhow::Error> {
    let data_file = PathBuf::from_str(TASK_DATA_FILE).unwrap();
    let task = task_query.build();
    trace!("{:?}", task.get_args());
    write_to_file(task, data_file)

}

fn write_to_file(mut task: Command, data_file: PathBuf) -> Result<PathBuf, anyhow::Error>{
    match task
        .output()
        .and_then(|v| {
            // write the output to file,
            fs::write(&data_file, v.stdout)
        }) {
        Ok(_) => Ok(data_file),
        Err(e) => {
            error!("{}", e);
            anyhow::bail!("Failed to read tasks")
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct TaskUUID(String);

fn read_task_file(task_query: TaskQuery) -> Result<IndexMap<TaskUUID, Task>, anyhow::Error> {
    let data_file = fetch_task_from_cmd(&task_query)?;
    let content = fs::read_to_string(&data_file)?;
    let tasks: Vec<Task> = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => anyhow::bail!(e.to_string())
    };
    let mut hm = IndexMap::new();
    for task in tasks.iter() {
        hm.insert(TaskUUID(task.uuid.clone()), task.clone());
    }
    Ok(hm)
}

// what would happen
pub fn list_tasks(task_query: TaskQuery) -> Result<IndexMap<TaskUUID, Task>, anyhow::Error> {
    read_task_file(task_query)
}

// update a single task
pub fn update_tasks(task: TaskUpdateStatus) -> Result<(), anyhow::Error> {
    let t = TaskQuery::new(Params::default());
    let mut tasks = read_task_file(t)?;
    let t = match tasks.get_mut(&TaskUUID(task.uuid.clone())) {
        None => return anyhow::bail!("Matching task not found"),
        Some(t) => t
    };
    t.status = Some(task.status);
    let entry = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    t.end = Some(entry.clone());
    t.modified = Some(entry);

    let tasks_vec: Vec<Task> = tasks.values().cloned().collect();

    let data_file = PathBuf::from(TASK_DATA_FILE_EDIT);
    fs::write(&data_file, serde_json::to_string(&tasks_vec)?)?;
    match data_file.canonicalize()
        .and_then(|v| {
            Command::new("task")
                .arg("import")
                .arg(v.to_str().unwrap())
                .output()
        }) {
        Ok(o) if o.status.exit_ok().is_ok() => {
            info!("Synced with task");
            Ok(())
        }
        Err(e) => {
            error!("Failed to sync with task");
            return anyhow::bail!("Failed to sync");
        }
        Ok(o) => {
            error!("Not ok from task command {:?} {:?}", o, o.stderr);
            return anyhow::bail!("Failed to sync");
        }
    }
}

pub(crate) fn execute_command(cmd: String) -> Result<PathBuf, anyhow::Error>{
    let mut task = Command::new("task");
    let output_file = PathBuf::from(TASK_OUTPUT_FILE);
    let file = File::create(&output_file).unwrap();
    let stdio = Stdio::from(file);
    let params: Vec<&str> = cmd.split(" ").collect();
    if params.len() > 0 {
        for param in params.iter() {
            task.arg(param);
        }
    }
    trace!("{:?}", params);
    write_to_file(task, output_file)
}