use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, trace};
use std::process::Command;
use std::fs;
use std::str::FromStr;
use indexmap::IndexMap;
use serde_json::Value;

pub mod task_query_builder;

use task_query_builder::TaskQuery;
use crate::{TWGlobalState, TaskUpdateStatus, NewTask};

pub const TASK_DATA_FILE: &str = "data.json";
pub const TASK_DATA_FILE_EDIT: &str = "data_edit.json";
pub const TASK_OUTPUT_FILE: &str = "output.out";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Annotation {
    entry: String,
    description: String,
}


#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Task {
    pub id: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub uuid: Option<String>,
    pub urgency: Option<f64>,
    pub entry: Option<String>,
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
    #[serde(skip_serializing_if = "Option::is_none", rename = "UDA")]
    pub uda: Option<HashMap<String, Value>>,
}


pub fn fetch_task_from_cmd(
    task_query: &TaskQuery,
    editing: bool,
) -> Result<String, anyhow::Error> {
    let mut task = task_query.build();
    trace!("{:?}", task.get_args());
    return match task.output() {
        Ok(v) => Ok(String::from_utf8(v.stdout.to_vec())?),
        Err(e) => {
            error!("{}", e);
            anyhow::bail!("Failed to read tasks")
        }
    }
}

fn write_to_file(mut task: Command, data_file: PathBuf) -> Result<PathBuf, anyhow::Error> {
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

fn read_task_file(task_query: &TaskQuery, editing: bool) -> Result<IndexMap<TaskUUID, Task>, anyhow::Error> {
    let content = fetch_task_from_cmd(&task_query, editing)?;
    let tasks: Vec<Task> = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => anyhow::bail!(e.to_string())
    };
    let mut hm = IndexMap::new();
    for task in tasks.iter() {
        hm.insert(TaskUUID(task.uuid.as_ref().unwrap().clone()), task.clone());
    }
    Ok(hm)
}

pub fn task_undo_report() -> Result<String, anyhow::Error> {
    match Command::new("task")
        .arg("undo")
        .output() {
        Ok(o) => {
            let s = String::from_utf8(o.stdout).unwrap();
            Ok(s)
        }
        Err(e) => {
            error!("Failed to execute command: {}", e);
            anyhow::bail!("Failed to get undo report");
        }
    }
}

pub fn task_add(task: &NewTask) -> Result<(), anyhow::Error> {
    let mut cmd = Command::new("task");
    cmd
        .arg("add")
        .arg(task.description());
    // add the tags
    if let Some(tags) = task.tags() && tags != "" {
        for tag in tags.split(' ') {
            if !tag.starts_with('+') {
                cmd.arg(&format!("+{}", tag));
            } else {
                cmd.arg(&tag);
            }
        }
    }
    // add the project
    if let Some(project) = task.project() {
        if !project.starts_with("project:") {
            cmd.arg(&format!("project:{}", project));
        } else {
            cmd.arg(&project);
        }
    }

    if let Some(additional) = task.additional() {
        for a in additional.split(' ') {
            cmd.arg(&a);
        }
    }

    match cmd.output() {
        Ok(_o) => {
            info!("New task added");
            Ok(())
        }
        Err(e) => {
            error!("Failed to execute new task: {}", e);
            anyhow::bail!("Failed to add new task");
        }
    }
}

pub fn task_undo() -> Result<(), anyhow::Error> {
    match Command::new("task")
        .arg("rc.confirmation:off")
        .arg("undo")
        .output() {
        Ok(_o) => {
            info!("Task undo success");
            Ok(())
        }
        Err(e) => {
            error!("Failed to execute undo: {}", e);
            anyhow::bail!("Failed to undo");
        }
    }
}

// what would happen
pub fn list_tasks(task_query: &TaskQuery) -> Result<IndexMap<TaskUUID, Task>, anyhow::Error> {
    read_task_file(task_query, false)
}

pub fn run_modify_command(task_uuid: &str, cmd_text: &str) -> Result<(), anyhow::Error> {
    let mut task_cmd = Command::new("task");
    task_cmd.arg("modify").arg(task_uuid);
    cmd_text.split(' ').for_each(|v| {
        task_cmd.arg(v);
    });
    if let Err(e) = task_cmd.output() {
        error!("Failed to execute command: {}", e);
        anyhow::bail!("Failed to execute modify command");
    }
    Ok(())
}

pub fn run_annotate_command(task_uuid: &str, annotation: &str) -> Result<(), anyhow::Error> {
    let mut task_cmd = Command::new("task");
    task_cmd.arg("annotate").arg(task_uuid);
    annotation.split(' ').for_each(|v| {
        task_cmd.arg(v);
    });
    if let Err(e) = task_cmd.output() {
        error!("Failed to execute command: {}", e);
        anyhow::bail!("Failed to execute annotation command");
    }
    Ok(())
}

pub fn run_denotate_command(task_uuid: &str) -> Result<(), anyhow::Error> {
    let mut task_cmd = Command::new("task");
    task_cmd.arg(task_uuid).arg("denotate");
    if let Err(e) = task_cmd.output() {
        error!("Failed to execute command: {}", e);
        anyhow::bail!("Failed to execute denotate command");
    }
    Ok(())
}

// mark a task as done
pub fn mark_task_as_done(task: TaskUpdateStatus) -> Result<(), anyhow::Error> {
    let mut t = get_task_from_tw(&task.uuid)?;
    let entry = chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    t.modified = Some(entry.clone());
    t.status = Some(task.status);
    t.end = Some(entry);
    execute_update(t)
}

pub fn fetch_active_task() -> Result<Option<Task>, anyhow::Error> {
    // maybe another task is running? So stop all other tasks first
    match Command::new("task")
        .arg("+ACTIVE")
        .arg("export")
        .output() {
        Err(e) => {
            error!("No active task found: {}", e);
            anyhow::bail!("No active task found");
        }
        Ok(v) => {
            let n = String::from_utf8(v.stdout).unwrap();
            let res: Vec<Task> = serde_json::from_str(&n)?;
            if res.len() == 0 {
                Ok(None)
            } else {
                Ok(res.first().cloned())
            }
        }
    }
}

pub fn toggle_task_active(task_uuid: &str) -> Result<bool, anyhow::Error> {
    let t = get_task_from_tw(task_uuid)?;
    // maybe another task is running? So stop all other tasks first
    if let Err(e) = Command::new("task")
        .arg("+ACTIVE")
        .arg("stop")
        .output() {
        error!("Failed to stop any task: {}", e);
        anyhow::bail!("Failed to stop task");
    }
    let is_running = t.start.is_none();
    // the task was not running, so let's start it
    if is_running {
        if let Err(e) = Command::new("task")
            .arg(task_uuid)
            .arg("start")
            .output() {
            error!("Failed to start task: {}", e);
            anyhow::bail!("Failed to start task");
        }
    }
    // the task is now running
    Ok(is_running)
}

fn execute_update(t: Task) -> Result<(), anyhow::Error> {
    let tasks_vec: Vec<Task> = vec![t];
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
            error!("Failed to sync with task: {}", e);
            anyhow::bail!("Failed to sync");
        }
        Ok(o) => {
            error!("Not ok from task command {:?} {:?}", o, o.stderr);
            anyhow::bail!("Failed to sync");
        }
    }
}

fn get_task_from_tw(task_uuid: &str) -> Result<Task, anyhow::Error> {
    let mut p = TWGlobalState::default();
    p.filter = Some(task_uuid.to_string());
    let t = TaskQuery::all();
    let tasks = read_task_file(&t, true)?;
    match tasks.get(&TaskUUID(task_uuid.to_string())) {
        None => anyhow::bail!("Matching task not found"),
        Some(t) => Ok(t.clone())
    }
}

pub fn get_task_details(uuid: String) -> Result<Task, anyhow::Error> {
    info!("uuid: {}", uuid);
    let mut p = TWGlobalState::default();
    p.filter = Some(uuid.clone());
    let mut t = TaskQuery::empty();
    t.set_filter(&uuid);
    let tasks = read_task_file(&t, true)?;
    match tasks.get(&TaskUUID(uuid.clone())) {
        None => anyhow::bail!("Matching task not found"),
        Some(t) => Ok(t.clone())
    }
}

