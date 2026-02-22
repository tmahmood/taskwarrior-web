/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */

use crate::backend::serde::{task_date_format, task_date_format_mandatory, task_status_serde};
use crate::core::app::AppState;
use crate::core::errors::AppError;
use anyhow::{Error, bail};
use chrono::{DateTime, TimeZone, Utc, offset::LocalResult};
use serde::{Deserialize, Serialize};
use std::fs::DirEntry;
use std::{
    collections::HashMap,
    fmt::Display,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use taskchampion::{Operation, Operations, Replica, SqliteStorage, Uuid};
use tracing::{debug, error, info};

#[cfg(windows)]
const LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
const LINE_ENDING: &str = "\n";

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

impl Display for TaskProperties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::DESCRIPTION => "description".to_string(),
            Self::DUE => "due".to_string(),
            Self::MODIFIED => "modified".to_string(),
            Self::START => "start".to_string(),
            Self::STATUS => "status".to_string(),
            Self::PRIORITY => "priority".to_string(),
            Self::WAIT => "wait".to_string(),
            Self::END => "end".to_string(),
            Self::ENTRY => "entry".to_string(),
            Self::PROJECT => "project".to_string(),
        };
        write!(f, "{s}")
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
        x.iter().find(|p| p.to_string().eq(&chk_value)).map_or_else(
            || {
                Err(Error::msg(format!(
                    "Property {} is not a reserved property.",
                    &value
                )))
            },
            |x| Ok(x.to_owned()),
        )
    }
}

#[must_use]
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
/// `OnAdd` requires executable script named with starting `on-add` and is executed
/// on new definition of a task.
/// `OnModify` requires executable script named with starting `on-modify` and is executed
/// whenever a task is changed.
#[derive(Clone, Debug)]
pub enum TaskEvent {
    OnAdd,
    OnModify,
}

impl Display for TaskEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::OnAdd => "on-add".to_string(),
            Self::OnModify => "on-modify".to_string(),
        };
        write!(f, "{s}")
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
        value.0.parse::<i64>().map_or(None, |ts| {
            let LocalResult::Single(result) = Utc.timestamp_opt(ts, 0) else {
                unreachable!("We're requesting UTC so daylight saving time isn't a factor.")
            };
            Some(result)
        })
    }
}

impl TcDateConverter {
    fn convert_to_datetime(value: Option<Self>) -> Option<DateTime<Utc>> {
        value.and_then(Into::into)
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
            .filter(taskchampion::Tag::is_user)
            .map(|p| p.to_string())
            .collect();
        let deps: Vec<Uuid> = value.get_dependencies().collect();
        let mut annotations: Vec<Annotation> =
            value.get_annotations().map(Annotation::from).collect();
        annotations.sort();
        annotations.reverse();
        let uda: HashMap<String, String> = value
            .get_user_defined_attributes()
            .map(|p| (p.0.to_string(), p.1.to_string()))
            .collect();

        Self {
            id: None,
            uuid: value.get_uuid(),
            description: value.get_description().to_string(),
            tags: Some(tags),
            depends: Some(deps),
            end: value.get_value("start").map(ToString::to_string),
            project: value
                .get_value(TaskProperties::PROJECT.to_string())
                .map(ToString::to_string),
            urgency: value
                .get_value("urgency")
                .map(|p| p.parse::<f64>().unwrap_or_default()),
            // timestamp
            entry: TcDateConverter::convert_to_datetime(value.get_value("entry").map(Into::into)),
            // timestamp
            start: TcDateConverter::convert_to_datetime(value.get_value("start").map(Into::into)),
            // timestamp
            until: TcDateConverter::convert_to_datetime(value.get_value("until").map(Into::into)),
            scheduled: TcDateConverter::convert_to_datetime(
                value.get_value("scheduled").map(Into::into),
            ),
            annotations: Some(annotations),
            due: value.get_due(),
            modified: value.get_modified(),
            status: Some(value.get_status()),
            wait: value.get_wait(),
            priority: Some(value.get_priority().to_string()),
            recur: value.get_value("recur").map(ToString::to_string),
            mask: value.get_value("mask").map(ToString::to_string),
            imask: value
                .get_value("imask")
                .map(|p| p.parse::<f64>().unwrap_or_default()),
            parent: value.get_value("parent").map(ToString::to_string),
            uda: Some(uda),
        }
    }
}

impl Task {
    pub const fn set_id(&mut self, id: Option<i64>) {
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

/// Get replica access to the taskchampion database stored in `taskdb`
/// # Errors
///
/// Will return error if unable to access replica
pub async fn get_replica(taskdb: &Path) -> Result<Replica<SqliteStorage>, anyhow::Error> {
    // Create a new Replica, storing data on disk.
    let storage = SqliteStorage::new(
        taskdb.to_path_buf(),
        taskchampion::storage::AccessMode::ReadWrite,
        true,
    )
    .await?;
    Ok(Replica::new(storage))
}

/// Executes hook scripts based on the type of task events.
///
/// # Behavior
///
/// - If `hooks_dir` is `None`, or if the directory does not exist, the function
///   logs a warning and exits early.
/// - Hook scripts are identified as executable files with names that start with
///   the string representation of the `event_type`.
///
/// # Notes
///
/// - This function uses Unix-style file permission checks to determine
///   whether a file is executable.
/// - Hook scripts must conform to the naming conventions (beginning with the
///   task event type) to be picked up for execution.
///
pub fn execute_hooks(
    hooks_dir: &Option<PathBuf>,
    event_type: &TaskEvent,
    old: &Option<Task>,
    new: &Option<Task>,
) {
    let old_task = old
        .as_ref()
        .map(|f| serde_json::to_string(&f).unwrap_or_default())
        .unwrap_or_default();

    let new_task = new
        .as_ref()
        .map(|f| serde_json::to_string(&f).unwrap_or_default())
        .unwrap_or_default();

    let mut args = String::new();
    if matches!(event_type, TaskEvent::OnModify) {
        args.push_str(&old_task);
        args.push_str(LINE_ENDING);
    }
    args.push_str(&new_task);
    args.push_str(LINE_ENDING);

    // find scripts and execute.
    let Some(hooks_dir) = hooks_dir.as_ref() else {
        tracing::warn!("Hooks directory is not set, skipping hook execution.");
        return;
    };

    let Ok(paths) = std::fs::read_dir(hooks_dir) else {
        tracing::warn!(
            "Failed to read Hooks directory {:?}, skipping hook execution.",
            hooks_dir
        );
        return;
    };
    paths
        .filter_map(|dir_entry| {
            let Ok(entry) = dir_entry else {
                return None;
            };

            let Ok(meta) = entry.metadata() else {
                return None;
            };

            let permissions = meta.permissions();
            let is_executable = permissions.mode() & 0o111 != 0;

            if !meta.is_file() || !is_executable {
                return None;
            }

            if let Some(name) = entry.file_name().to_str()
                && name.starts_with(&event_type.to_string())
            {
                return Some(entry);
            }
            None
        })
        .for_each(|entry| {
            execute_hook_file(&args, &entry);
        });
}

fn execute_hook_file(args: &str, entry: &DirEntry) {
    debug!(
        "Hook {:?} will be executed with stdin: {}",
        &entry.file_name().to_str(),
        &args
    );
    Command::new(entry.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_or_else(
            |e| {
                error!(
                    "Hook {:?} failed to spawn with error {}",
                    &entry.file_name().to_str(),
                    e.to_string()
                );
            },
            |mut child| {
                let child_stdin = child.stdin.as_mut().unwrap();
                let _ = child_stdin.write_all(args.as_bytes());

                let cmd = child.wait_with_output();
                match cmd {
                    Ok(o) => {
                        let output = [o.stdout.as_slice()].concat();
                        let output = String::from_utf8(output)
                            .unwrap_or_else(|_| "Output no valid UTF-8".to_string());
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
            },
        );
}

/// Gives a dedup list of projects in the given taskchampion data source.
/// # Errors
///
/// Will return error if unable to access replica
pub async fn get_project_list(taskdb: &Path) -> Result<Vec<String>, AppError> {
    let mut replica = get_replica(taskdb).await?;
    let mut x = replica
        .all_task_data()
        .await?
        .values()
        .filter_map(|task_data| {
            if task_data
                .get(TaskProperties::PROJECT.to_string())
                .is_some_and(|v| !v.is_empty())
            {
                task_data.get(TaskProperties::PROJECT.to_string())
            } else {
                None
            }
        })
        .map(Into::into)
        .collect::<Vec<_>>();
    x.sort();
    x.dedup();
    Ok(x)
}

/// Get a list of tags used in any of current worksets replica content.
#[allow(dead_code)]
async fn get_tag_list(taskdb: &Path) -> Result<Vec<String>, AppError> {
    let mut replica = get_replica(taskdb).await?;
    let mut tags: Vec<String> = vec![];

    for task in replica.all_task_data().await? {
        for f in task.1.properties().filter(|x| x.starts_with("tag_")) {
            let tag_name = f.strip_prefix("tag_").unwrap_or(f);
            tags.push(tag_name.to_owned());
        }
    }
    tags.sort();
    tags.dedup();

    Ok(tags)
}

pub async fn get_undo_operations(
    taskdb: &Path,
) -> Result<HashMap<Uuid, Vec<TaskOperation>>, AppError> {
    let mut replica = get_replica(taskdb).await?;
    let ops = replica.get_undo_operations().await?;
    let mut converted_ops: HashMap<Uuid, Vec<TaskOperation>> = HashMap::new();
    for e in ops {
        let converted_entry = match e {
            Operation::Create { uuid } => Some((
                uuid,
                TaskOperation {
                    operation: "Create".to_string(),
                    uuid,
                    property: None,
                    old_value: None,
                    value: None,
                    timestamp: None,
                    old_task: None,
                    is_tag_change: false,
                },
            )),
            Operation::Delete { uuid, old_task } => Some((
                uuid,
                TaskOperation {
                    operation: "Delete".to_string(),
                    uuid,
                    property: None,
                    old_value: None,
                    value: None,
                    timestamp: None,
                    old_task: Some(old_task),
                    is_tag_change: false,
                },
            )),
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
                Some((
                    uuid,
                    TaskOperation {
                        operation: "Modified".to_string(),
                        uuid,
                        property: Some(property),
                        old_value,
                        value,
                        timestamp: Some(timestamp),
                        old_task: None,
                        is_tag_change: *is_tag_change,
                    },
                ))
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

pub async fn get_task(taskdb: &Path, task_id: Uuid) -> Result<Option<Task>, anyhow::Error> {
    let mut replica = get_replica(taskdb).await?;
    let idx: Option<i64> = replica
        .working_set()
        .await?
        .by_uuid(task_id)
        .map(|p| Some(p as i64))
        .unwrap_or(None);
    let maybe_task = replica.get_task(task_id).await.map(|maybe_task| {
        maybe_task.map_or_else(
            || None,
            |task_found| {
                let mut task = Task::from(task_found);
                task.set_id(idx);
                Some(task)
            },
        )
    })?;
    Ok(maybe_task)
}

pub async fn denotate_task(
    task_id: Uuid,
    anno: &Annotation,
    app_state: &AppState,
) -> Result<Task, anyhow::Error> {
    let mut replica = get_replica(&app_state.task_storage_path).await?;
    let mut ops = Operations::new();
    let maybe_task = replica.get_task(task_id).await?;
    let Some(mut task) = maybe_task else {
        bail!("Failed to get task");
    };
    let old_task = task.clone();
    ops.push(taskchampion::Operation::UndoPoint);
    task.remove_annotation(anno.entry, &mut ops)?;

    match replica.commit_operations(ops).await {
        Ok(()) => {
            info!(
                "Removed task {} annotation {}",
                task_id.to_string(),
                anno.entry
            );
            // execute hooks.
            let ct: crate::backend::task::Task = task.into();
            execute_hooks(
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
