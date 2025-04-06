use chrono::Utc;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::str::FromStr;
use taskchampion::{Operations, Status, Tag, Uuid};
use tracing::{debug, error, info, trace};

pub mod task_query_builder;

use crate::backend::task::{
    convert_task_status, execute_hooks, get_replica, TaskEvent, TaskProperties,
};
use crate::core::app::AppState;
use crate::core::errors::{FieldError, FormValidation};
use crate::{NewTask, TWGlobalState, TaskUpdateStatus};
use task_query_builder::TaskQuery;

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
    pub uuid: String,
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

pub fn fetch_task_from_cmd(task_query: &TaskQuery) -> Result<String, anyhow::Error> {
    let mut task = task_query.build();
    trace!("{:?}", task.get_args());
    return match task.output() {
        Ok(v) => Ok(String::from_utf8(v.stdout.to_vec())?),
        Err(e) => {
            error!("{}", e);
            anyhow::bail!("Failed to read tasks")
        }
    };
}

#[derive(Debug, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct TaskUUID(String);

impl From<TaskUUID> for Uuid {
    fn from(val: TaskUUID) -> Self {
        Uuid::from_str(&val.0).expect("No valid uuid given")
    }
}

fn read_task_file(task_query: &TaskQuery) -> Result<IndexMap<TaskUUID, Task>, anyhow::Error> {
    let content = fetch_task_from_cmd(&task_query)?;
    let tasks: Vec<Task> = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => anyhow::bail!(e.to_string()),
    };
    let mut hm = IndexMap::new();
    for task in tasks.iter() {
        hm.insert(TaskUUID(task.uuid.clone()), task.clone());
    }
    Ok(hm)
}

/// Create a new task
/// Requires corresponding task information and path to the taskchampion directory.
///
/// The data will be evaluated and a response will be provided via FormValidation.
pub fn task_add(task: &NewTask, app_state: &AppState) -> Result<(), FormValidation> {
    let mut validation_result = FormValidation::default();
    let mut replica = get_replica(&app_state.task_storage_path)
        .map_err(|err| <anyhow::Error as Into<FormValidation>>::into(err))?;
    let uuid = Uuid::new_v4();
    let mut ops = Operations::new();
    ops.push(taskchampion::Operation::UndoPoint);

    let mut t = replica
        .create_task(uuid, &mut ops)
        .map_err(|err| <taskchampion::Error as Into<FormValidation>>::into(err))?;
    match t
        .set_description(task.description.to_string(), &mut ops)
        .map_err(|p| FieldError {
            field: TaskProperties::DESCRIPTION.to_string(),
            message: p.to_string(),
        }) {
        Ok(_) => {
            // Check if it was field. This is a mandatory field!
            if task.description.trim().is_empty() {
                validation_result.push(FieldError {
                    field: TaskProperties::DESCRIPTION.to_string(),
                    message: "Description field is mandatory".to_string(),
                })
            }
        }
        Err(e) => validation_result.push(e),
    };
    match t
        .set_status(Status::Pending, &mut ops)
        .map_err(|p| FieldError {
            field: TaskProperties::STATUS.to_string(),
            message: p.to_string(),
        }) {
        Ok(_) => (),
        Err(e) => validation_result.push(e),
    };
    match t
        .set_entry(Some(Utc::now()), &mut ops)
        .map_err(|p| FieldError {
            field: TaskProperties::ENTRY.to_string(),
            message: p.to_string(),
        }) {
        Ok(_) => (),
        Err(e) => validation_result.push(e),
    };
    if let Some(tags) = task.tags()
        && tags.trim().len() > 0
    {
        for tag in tags.split(&[' ', '+', '-']) {
            if !tag.trim().is_empty() {
                match &Tag::from_str(tag).map_err(|p| FieldError {
                    field: "tags".to_string(),
                    message: p.to_string(),
                }) {
                    Ok(tag) => match t.add_tag(tag, &mut ops).map_err(|p| FieldError {
                        field: "tags".to_string(),
                        message: p.to_string(),
                    }) {
                        Ok(_) => (),
                        Err(e) => validation_result.push(e),
                    },
                    Err(e) => validation_result.push(e.to_owned()),
                };
            }
        }
    }
    if let Some(project) = task.project() {
        match t
            .set_value(
                TaskProperties::PROJECT.to_string(),
                Some(project.to_string()),
                &mut ops,
            )
            .map_err(|p| FieldError {
                field: TaskProperties::PROJECT.to_string().to_string(),
                message: p.to_string(),
            }) {
            Ok(_) => (),
            Err(e) => validation_result.push(e),
        };
    }

    if let Some(additional) = task.additional() {
        for a in additional.split(' ') {
            let b1 = a
                .split_once(':')
                .map_or((a, None), |p| (p.0, Some(p.1.to_string())));
            if let Ok(_) = TaskProperties::try_from(b1.0) {
                match t.set_value(b1.0, b1.1, &mut ops).map_err(|p| FieldError {
                    field: "additional".to_string(),
                    message: p.to_string(),
                }) {
                    Ok(_) => (),
                    Err(e) => validation_result.push(e),
                };
            } else {
                match t
                    .set_user_defined_attribute(b1.0, b1.1.unwrap_or("".to_string()), &mut ops)
                    .map_err(|p| FieldError {
                        field: "additional".to_string(),
                        message: p.to_string(),
                    }) {
                    Ok(_) => (),
                    Err(e) => validation_result.push(e),
                };
            }
        }
    }

    match validation_result.is_success() {
        true => {
            // Commit those operations to storage.
            match replica.commit_operations(ops) {
                Ok(_) => {
                    info!("New task {} added", uuid.to_string());
                    // execute hooks.
                    let ct: crate::backend::task::Task = t.into();
                    let _ = execute_hooks(
                        &app_state.task_hooks_path,
                        &TaskEvent::OnAdd,
                        &None,
                        &Some(ct),
                    );
                    Ok(())
                }
                Err(e) => {
                    error!(
                        "Could not create task {}, error: {}",
                        uuid.to_string(),
                        e.to_string()
                    );
                    Err(e.into())
                }
            }
        }
        false => Err(validation_result.into()),
    }
}

pub fn task_undo() -> Result<(), anyhow::Error> {
    match Command::new("task")
        .arg("rc.confirmation:off")
        .arg("undo")
        .output()
    {
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
    read_task_file(task_query)
}

pub fn run_modify_command(task_uuid: Uuid, cmd_text: &str) -> Result<(), anyhow::Error> {
    let mut task_cmd = Command::new("task");
    task_cmd.arg("modify").arg(task_uuid.to_string());
    cmd_text.split(' ').for_each(|v| {
        task_cmd.arg(v);
    });
    if let Err(e) = task_cmd.output() {
        error!("Failed to execute command: {}", e);
        anyhow::bail!("Failed to execute modify command");
    }
    Ok(())
}

pub fn run_annotate_command(task_uuid: Uuid, annotation: &str) -> Result<(), anyhow::Error> {
    let mut task_cmd = Command::new("task");
    task_cmd.arg("annotate").arg(task_uuid.to_string());
    annotation.split(' ').for_each(|v| {
        task_cmd.arg(v);
    });
    if let Err(e) = task_cmd.output() {
        error!("Failed to execute command: {}", e);
        anyhow::bail!("Failed to execute annotation command");
    }
    Ok(())
}

pub fn run_denotate_command(task_uuid: Uuid) -> Result<(), anyhow::Error> {
    let mut task_cmd = Command::new("task");
    task_cmd.arg(task_uuid.to_string()).arg("denotate");
    if let Err(e) = task_cmd.output() {
        error!("Failed to execute command: {}", e);
        anyhow::bail!("Failed to execute denotate command");
    }
    Ok(())
}

/// Change task status
/// Mostly it switches between pending and completed.
/// In any case, if the status is changed, the timer is stopped if active.
pub fn change_task_status(
    task: TaskUpdateStatus,
    app_state: &AppState,
) -> Result<(), anyhow::Error> {
    let mut replica = get_replica(&app_state.task_storage_path)?;
    let mut ops = Operations::new();
    ops.push(taskchampion::Operation::UndoPoint);

    let mut t = replica
        .get_task(task.uuid)
        .unwrap()
        .expect("Task does not exist");

    let old_task = t.clone();
    let task_status = convert_task_status(task.status);

    // Stop tasks.
    if t.is_active() {
        t.stop(&mut ops)?;
    }

    t.set_status(task_status, &mut ops)?;

    // Commit those operations to storage.
    match replica.commit_operations(ops) {
        Ok(_) => {
            info!("Task {} completed", task.uuid.to_string());

            // execute hooks.
            let ct: crate::backend::task::Task = t.into();
            let _ = execute_hooks(
                &app_state.task_hooks_path,
                &TaskEvent::OnModify,
                &Some(old_task.into()),
                &Some(ct),
            );
            Ok(())
        }
        Err(e) => {
            error!(
                "Could not create task {}, error: {}",
                task.uuid.to_string(),
                e.to_string()
            );
            Err(e.into())
        }
    }
}

pub fn fetch_active_task() -> Result<Option<Task>, anyhow::Error> {
    // maybe another task is running? So stop all other tasks first
    match Command::new("task").arg("+ACTIVE").arg("export").output() {
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

pub fn toggle_task_active(task_uuid: Uuid, app_state: &AppState) -> Result<bool, anyhow::Error> {
    let mut replica = get_replica(&app_state.task_storage_path)?;
    let mut ops = Operations::new();
    ops.push(taskchampion::Operation::UndoPoint);

    let mut t = replica
        .get_task(task_uuid)
        .unwrap()
        .expect("Task does not exist");

    let old_task = t.clone();
    let mut changed_tasks: Vec<(taskchampion::Task, taskchampion::Task)> = Vec::new();

    // Stop all active tasks.
    for mut single_task in replica.all_tasks()? {
        if single_task.1.is_active() {
            let old = single_task.1.clone();
            single_task.1.stop(&mut ops)?;
            let new = single_task.1.clone();
            changed_tasks.push((old, new));
        }
    }
    t.start(&mut ops)?;
    changed_tasks.push((old_task, t.clone()));

    // Commit those operations to storage.
    match replica.commit_operations(ops) {
        Ok(_) => {
            info!("Task {} started", task_uuid.to_string());
            // execute hooks.
            for t in changed_tasks {
                let _ = execute_hooks(
                    &app_state.task_hooks_path,
                    &TaskEvent::OnModify,
                    &Some(t.0.into()),
                    &Some(t.1.into()),
                );
            }
            Ok(t.is_active())
        }
        Err(e) => {
            error!(
                "Could not start task {}, error: {}",
                task_uuid.to_string(),
                e.to_string()
            );
            Err(e.into())
        }
    }
}

pub fn get_task_details(uuid: String) -> Result<Task, anyhow::Error> {
    debug!("uuid: {}", uuid);
    let mut p = TWGlobalState::default();
    p.filter = Some(uuid.clone());
    let mut t = TaskQuery::empty();
    t.set_filter(&uuid);
    let tasks = read_task_file(&t)?;
    match tasks.get(&TaskUUID(uuid.clone())) {
        None => anyhow::bail!("Matching task not found"),
        Some(t) => Ok(t.clone()),
    }
}

/// Parse task settings
pub fn task_show() -> Result<IndexMap<String, String>, anyhow::Error> {
    let mut settings = IndexMap::<String, String>::default();
    let rr = Command::new("task").arg("show").output()?.stdout;
    String::from_utf8(rr)?.lines().for_each(|line| {
        let mut ss = line.split_once(": ");
        if let Some((key, val)) = ss {
            settings.insert(key.trim().to_string(), val.trim().to_string());
        } else {
            error!("FAIL: {}", line);
            ss = line.split_once(" ");
            if let Some((key, val)) = ss {
                settings.insert(key.trim().to_string(), val.trim().to_string());
            } else {
                error!("TOTAL FAIL: {}", line);
            }
        }
    });
    Ok(settings)
}

pub const TAG_KEYWORDS: [&str; 4] = ["next", "pending", "completed", "new"];

pub fn is_tag_keyword(tag: &str) -> bool {
    TAG_KEYWORDS.contains(&tag)
}

pub fn is_a_tag(tag: &str) -> bool {
    is_tag_keyword(tag) || tag.starts_with("+")
}

pub struct TaskViewDataRetType {
    pub tasks: IndexMap<TaskUUID, Task>,
    pub tag_map: HashMap<String, String>,
    pub shortcuts: HashSet<String>,
    pub task_list: Vec<Task>,
    pub task_shortcut_map: HashMap<String, String>,
}
