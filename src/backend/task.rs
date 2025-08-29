/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */


use std::{
    collections::HashMap, io::Write, os::unix::fs::PermissionsExt, path::PathBuf, process::{Command, Stdio}
};

use anyhow::Error;
use chrono::{offset::LocalResult, DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use taskchampion::{Operation, Operations, Replica, StorageConfig, Uuid};
use tracing::{debug, error, info};
use crate::{backend::serde::{task_date_format, task_date_format_mandatory, task_status_serde}, core::app::AppState};

use crate::core::errors::AppError;

#[cfg(windows)]
const LINE_ENDING: &'static str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &'static str = "\n";

#[derive(Clone, Debug)]
pub enum TaskProperties {
    DESCRIPTION,
    DUE,
    MODIFIED,
    START,
    STATUS,
    PRIORITY,
    WAIT,
    END,
    ENTRY,
    PROJECT,
}

impl ToString for TaskProperties {
    fn to_string(&self) -> String {
        match self {
            TaskProperties::DESCRIPTION => "description".to_string(),
            TaskProperties::DUE => "due".to_string(),
            TaskProperties::MODIFIED => "modified".to_string(),
            TaskProperties::START => "start".to_string(),
            TaskProperties::STATUS => "status".to_string(),
            TaskProperties::PRIORITY => "priority".to_string(),
            TaskProperties::WAIT => "wait".to_string(),
            TaskProperties::END => "end".to_string(),
            TaskProperties::ENTRY => "entry".to_string(),
            TaskProperties::PROJECT => "project".to_string(),
        }
    }
}

impl TryFrom<&str> for TaskProperties {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let chk_value = value.to_lowercase();
        let x = vec![
            Self::DESCRIPTION,
            Self::DUE,
            Self::MODIFIED,
            Self::START,
            Self::STATUS,
            Self::PRIORITY,
            Self::WAIT,
            Self::END,
            Self::ENTRY,
            Self::PROJECT,
        ];
        match x.iter().find(|p| p.to_string().eq(&chk_value)) {
            Some(x) => Ok(x.to_owned()),
            None => Err(Error::msg(format!(
                "Property {} is not a reserved property.",
                &value
            ))),
        }
    }
}

pub fn convert_task_status(task_status: &str) -> taskchampion::Status {
    match task_status {
        "pending" => taskchampion::Status::Pending,
        "completed" => taskchampion::Status::Completed,
        "deleted" => taskchampion::Status::Deleted,
        "recurring" => taskchampion::Status::Recurring,
        &_ => taskchampion::Status::Unknown(task_status.into()),
    }
}

/// Supported hook events based on taskwarrior definitions.
///
/// OnAdd requires executable script named with starting `on-add` and is executed
/// on new definition of a task.
/// OnModify requires executable script named with starting `on-modify` and is executed
/// whenever a task is changed.
#[derive(Clone, Debug)]
pub enum TaskEvent {
    OnAdd,
    OnModify,
}

impl ToString for TaskEvent {
    fn to_string(&self) -> String {
        match self {
            TaskEvent::OnAdd => "on-add".to_string(),
            TaskEvent::OnModify => "on-modify".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct TcDateConverter(String);

impl From<&str> for TcDateConverter {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<TcDateConverter> for Option<DateTime<Utc>> {
    fn from(value: TcDateConverter) -> Self {
        if let Ok(ts) = value.0.parse::<i64>() {
            let result = match Utc.timestamp_opt(ts, 0) {
                LocalResult::Single(tz) => tz,
                // The other two variants are None and Ambiguous, which both are caused by DST.
                _ => {
                    unreachable!("We're requesting UTC so daylight saving time isn't a factor.")
                }
            };
            Some(result)
        } else {
            None
        }
    }
}

impl TcDateConverter {
    fn convert_to_datetime(value: Option<Self>) -> Option<DateTime<Utc>> {
        if let Some(val_ok) = value {
            val_ok.into()
        } else {
            None
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Annotation {
    #[serde(with = "task_date_format_mandatory")]
    entry: DateTime<Utc>,
    description: String,
}

impl From<taskchampion::Annotation> for Annotation {
    fn from(value: taskchampion::Annotation) -> Self {
        Self {
            entry: value.entry,
            description: value.description,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct Task {
    // id is the relative number within the working set!
    // to be retrieved with replica.working_set
    pub id: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    pub uuid: Uuid,
    pub urgency: Option<f64>,
    #[serde(default, with = "task_date_format")]
    pub entry: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, with = "task_date_format")]
    pub start: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, with = "task_date_format")]
    pub until: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, with = "task_date_format")]
    pub scheduled: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, with = "task_date_format")]
    pub due: Option<DateTime<Utc>>,
    #[serde(default, with = "task_date_format")]
    pub modified: Option<DateTime<Utc>>,
    #[serde(default, with = "task_status_serde")]
    pub status: Option<taskchampion::Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, with = "task_date_format")]
    pub wait: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depends: Option<Vec<Uuid>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recur: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imask: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "UDA")]
    pub uda: Option<HashMap<String, String>>,
}

impl From<taskchampion::Task> for Task {
    fn from(value: taskchampion::Task) -> Self {
        let tags: Vec<String> = value
            .get_tags()
            .filter(|p| p.is_user())
            .map(|p| p.to_string())
            .collect();
        let deps: Vec<Uuid> = value.get_dependencies().collect();
        let mut annotations: Vec<Annotation> = value
            .get_annotations()
            .map(|p| Annotation::from(p))
            .collect();
        annotations.sort();
        annotations.reverse();
        let uda: HashMap<String, String> = value
            .get_user_defined_attributes()
            .map(|p| (p.0.to_string(), p.1.to_string()))
            .collect();

        Task {
            id: None,
            uuid: value.get_uuid(),
            description: value.get_description().to_string(),
            tags: Some(tags),
            depends: Some(deps),
            end: value.get_value("start").map(|p| p.to_string()),
            project: value
                .get_value(TaskProperties::PROJECT.to_string())
                .map(|p| p.to_string()),
            urgency: value
                .get_value("urgency")
                .map(|p| p.parse::<f64>().unwrap_or_default()),
            // timestamp
            entry: TcDateConverter::convert_to_datetime(value.get_value("entry").map(|p| p.into())),
            // timestamp
            start: TcDateConverter::convert_to_datetime(value.get_value("start").map(|p| p.into())),
            // timestamp
            until: TcDateConverter::convert_to_datetime(value.get_value("until").map(|p| p.into())),
            scheduled: TcDateConverter::convert_to_datetime(
                value.get_value("scheduled").map(|p| p.into()),
            ),
            annotations: Some(annotations),
            due: value.get_due(),
            modified: value.get_modified(),
            status: Some(value.get_status()),
            wait: value.get_wait(),
            priority: Some(value.get_priority().to_string()),
            recur: value.get_value("recur").map(|p| p.to_string()),
            mask: value.get_value("mask").map(|p| p.to_string()),
            imask: value
                .get_value("imask")
                .map(|p| p.parse::<f64>().unwrap_or_default()),
            parent: value.get_value("parent").map(|p| p.to_string()),
            uda: Some(uda),
        }
    }
}

impl Task {
    pub fn set_id(&mut self, id: Option<i64>) {
        self.id = id;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOperation {
    pub operation: String,
    pub uuid: Uuid,
    pub property: Option<String>,
    pub old_value: Option<String>,
    pub value: Option<String>,
    pub timestamp: Option<DateTime<Utc>>,
    pub old_task: Option<HashMap<String, String>>,
    pub is_tag_change: bool,
}

/// Get a replica access to the taskchampion database stored in `taskdb`
pub fn get_replica(taskdb: &PathBuf) -> Result<Replica, anyhow::Error> {
    // Create a new Replica, storing data on disk.
    let storage = StorageConfig::OnDisk {
        taskdb_dir: taskdb.to_path_buf(),
        create_if_missing: true,
        access_mode: taskchampion::storage::AccessMode::ReadWrite,
    }
    .into_storage()?;
    Ok(Replica::new(storage))
}

/// Execute all hooks compliant to taskwarrior specification.
pub fn execute_hooks(
    hooks_dir: &Option<PathBuf>,
    event_type: &TaskEvent,
    old: &Option<Task>,
    new: &Option<Task>,
) -> Result<(), anyhow::Error> {
    if hooks_dir.is_none() {
        return Ok(());
    }

    let old_task = old
        .as_ref()
        .map(|f| serde_json::to_string(&f).unwrap_or("".to_string()))
        .unwrap_or("".to_string());
    let new_task = new
        .as_ref()
        .map(|f| serde_json::to_string(&f).unwrap_or("".to_string()))
        .unwrap_or("".to_string());

    let mut args = String::new();
    match event_type {
        TaskEvent::OnAdd => {}
        TaskEvent::OnModify => {
            args.push_str(&old_task);
            args.push_str(LINE_ENDING);
        }
    };
    args.push_str(&new_task);
    args.push_str(LINE_ENDING);

    // find scripts and execute.
    let hooks_dir = hooks_dir.as_ref().unwrap();
    let paths = std::fs::read_dir(&hooks_dir)?;
    for path in paths {
        if let Ok(entry) = path {
            if let Ok(meta) = entry.metadata() {
                let permissions = meta.permissions();
                let is_executable = permissions.mode() & 0o111 != 0;
                if meta.is_file()
                    && entry
                        .file_name()
                        .to_str()
                        .unwrap()
                        .starts_with(&event_type.to_string())
                    && is_executable
                {
                    debug!(
                        "Hook {:?} will be executed with stdin: {}",
                        &entry.file_name().to_str(),
                        &args
                    );
                    let child = Command::new(entry.path())
                        .stdin(Stdio::piped())
                        .stdout(Stdio::piped())
                        .spawn();
                    match child {
                        Ok(mut child) => {
                            let child_stdin = child.stdin.as_mut().unwrap();
                            let _ = child_stdin.write_all(args.as_bytes());

                            let cmd = child.wait_with_output();
                            match cmd {
                                Ok(o) => {
                                    let output = [o.stdout.as_slice()].concat();
                                    let output = String::from_utf8(output)
                                        .unwrap_or("Output no valid UTF-8".to_string());
                                    info!(
                                        "Hook {:?} called, exit status: {}",
                                        &entry.file_name().to_str(),
                                        o.status
                                    );
                                    debug!(
                                        "Hook {:?} output was {:?}",
                                        &entry.file_name().to_str(),
                                        output
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Hook {:?} failed with error {}",
                                        &entry.file_name().to_str(),
                                        e.to_string()
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            error!(
                                "Hook {:?} failed with error {}",
                                &entry.file_name().to_str(),
                                e.to_string()
                            );
                        }
                    };
                }
            }
        }
    }

    Ok(())
}

/// Gives a dedup list of projects in the given taskchampion data source.
pub fn get_project_list(taskdb: &PathBuf) -> Result<Vec<String>, AppError> {
    let mut replica = get_replica(taskdb)?;
    let mut x: Vec<String> = replica
        .all_task_data()?
        .values()
        .map(|f| f.get(TaskProperties::PROJECT.to_string()))
        .filter(|p| p.is_some_and(|f| !f.is_empty()))
        .map(|p| p.expect("Value given!").to_string())
        .collect();
    x.sort();
    x.dedup();

    Ok(x)
}

/// Get a list of tags used in any of current worksets replica content.
#[allow(dead_code)]
fn get_tag_list(taskdb: &PathBuf) -> Result<Vec<String>, AppError> {
    let mut replica = get_replica(taskdb)?;
    let mut tags: Vec<String> = vec![];

    for task in replica.all_task_data()? {
        for f in task.1.properties().filter(|x| x.starts_with("tag_")) {
            let tag_name = f.strip_prefix("tag_").unwrap_or(f);
            tags.push(tag_name.to_owned());
        }
    }
    tags.sort();
    tags.dedup();

    Ok(tags)
}


pub fn get_undo_operations(taskdb: &PathBuf) -> Result<HashMap<Uuid, Vec<TaskOperation>>, AppError> {
    let mut replica = get_replica(taskdb)?;
    let ops = replica.get_undo_operations()?;
    let mut converted_ops: HashMap<Uuid, Vec<TaskOperation>> = HashMap::new();
    for e in ops {
        let converted_entry = match e {
            Operation::Create { uuid } => Some((uuid.clone(), TaskOperation {
                operation: "Create".to_string(),
                uuid,
                property: None,
                old_value: None,
                value: None,
                timestamp: None,
                old_task: None,
                is_tag_change: false,
            })),
            Operation::Delete { uuid, old_task } => Some((uuid.clone(), TaskOperation {
                operation: "Delete".to_string(),
                uuid,
                property: None,
                old_value: None,
                value: None,
                timestamp: None,
                old_task: Some(old_task),
                is_tag_change: false,
            })),
            Operation::Update {
                uuid,
                property,
                old_value,
                value,
                timestamp,
            } => {
                let is_tag_change = &property.starts_with("tag_");
                let property = match is_tag_change {
                    true => property
                        .strip_prefix("tag_")
                        .unwrap_or(&property)
                        .to_string(),
                    false => property,
                };
                Some((uuid.clone(), TaskOperation {
                    operation: "Modified".to_string(),
                    uuid,
                    property: Some(property),
                    old_value,
                    value,
                    timestamp: Some(timestamp),
                    old_task: None,
                    is_tag_change: *is_tag_change,
                }))
            }
            Operation::UndoPoint => None,
        };
        if let Some(top) = converted_entry {
            if let Some(op_list) = converted_ops.get_mut(&top.0) {
                op_list.push(top.1);
            } else {
                converted_ops.insert(top.0, vec![top.1]);
            }
        }
    }
    Ok(converted_ops)
}

pub fn get_task(taskdb: &PathBuf, task_id: Uuid) -> Result<Option<Task>, anyhow::Error> {
    let mut replica = get_replica(taskdb)?;
    let idx: Option<i64> = replica.working_set().unwrap().by_uuid(task_id).map(|p| Some(p as i64)).unwrap_or(None);
    let x: Option<Task> = replica.get_task(task_id).map(|t| {
        if let Some(t_fine) = t {
            let mut task = Task::from(t_fine);
            task.set_id(idx);
            Some(task)
        } else {
            None
        }
    })?;
    Ok(x)
}


pub fn denotate_task(task_id: Uuid, anno: &Annotation, app_state: &AppState) -> Result<Task, anyhow::Error> {
    let mut replica = get_replica(&app_state.task_storage_path)?;
    let mut ops = Operations::new();
    let mut task = replica.get_task(task_id)?.expect("Could not found task");
    let old_task = task.clone();
    ops.push(taskchampion::Operation::UndoPoint);
    task.remove_annotation(anno.entry, &mut ops)?;

    match replica.commit_operations(ops) {
        Ok(_) => {
            info!("Removed task {} annotation {}", task_id.to_string(), anno.entry);
            // execute hooks.
            let ct: crate::backend::task::Task = task.into();
            let _ = execute_hooks(
                &app_state.task_hooks_path,
                &TaskEvent::OnModify,
                &Some(old_task.into()),
                &Some(ct.clone()),
            );
            Ok(ct)
        }
        Err(e) => {
            error!(
                "Could not create task {}, error: {}",
                task_id.to_string(),
                e.to_string()
            );
            Err(e.into())
        }
    }
}
