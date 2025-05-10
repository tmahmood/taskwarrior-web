use axum::extract::{Path, State};
use axum::http::header;
use axum::http::{Response, StatusCode};
use axum::Form;
use chrono::Utc;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::process::Command;
use std::str::FromStr;
use task_modify::{
    task_apply_depends, task_apply_description, task_apply_priority, task_apply_recur,
    task_apply_status, task_apply_tag_add, task_apply_tag_remove, task_apply_timestamps,
};
use taskchampion::{Operations, Replica, Status, Tag, Uuid};
use tera::Context;
use tracing::{debug, error, info, trace};

pub mod task_query_builder;

use crate::backend::task::{
    convert_task_status, denotate_task, execute_hooks, get_replica, get_task, Annotation,
    TaskEvent, TaskProperties,
};
use crate::core::app::{get_default_context, AppState};
use crate::core::config::CustomQuery;
use crate::core::errors::{FieldError, FormValidation};
use crate::core::utils::make_shortcut;
use crate::{NewTask, TWGlobalState, TaskUpdateStatus, TEMPLATES};
use task_query_builder::TaskQuery;

pub(crate) mod task_modify;

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

fn read_task_file(
    task_query: &TaskQuery,
) -> Result<IndexMap<TaskUUID, crate::backend::task::Task>, anyhow::Error> {
    let content = fetch_task_from_cmd(&task_query)?;
    let jd = &mut serde_json::Deserializer::from_str(&content);
    let result: Result<Vec<crate::backend::task::Task>, _> = serde_path_to_error::deserialize(jd);
    match result {
        Ok(_) => {}
        Err(err) => {
            let path = err.path().to_string();
            debug!("Received json: {:?}", &content);
            error!(path);
        }
    };
    let tasks: Vec<crate::backend::task::Task> = match serde_json::from_str(&content) {
        Ok(s) => s,
        Err(e) => anyhow::bail!(e.to_string()),
    };
    let mut hm = IndexMap::new();
    for task in tasks.iter() {
        hm.insert(TaskUUID(task.uuid.to_string()), task.clone());
    }
    Ok(hm)
}

fn parse_apply_additions(
    t: &mut taskchampion::Task,
    replica: &mut Replica,
    mut ops: &mut Vec<taskchampion::Operation>,
    additional: &String,
    validation_result: &mut FormValidation,
) {
    let task_additions = shell_words::split(&additional).map_err(|e| FieldError {
        field: "additional".to_string(),
        message: e.to_string(),
    });
    match task_additions {
        Ok(additional) => {
            debug!("Arguments: {:?}", additional);
            for a in additional {
                let b1 = a.split_once(':').map_or((a.trim().to_string(), None), |p| {
                    (p.0.trim().to_string(), Some(p.1.trim().to_string()))
                });

                // it might be a task operation if it starts with +/- without a value.
                if b1.0.starts_with("+") && b1.1.is_none() {
                    task_apply_tag_add(t, ops, validation_result, b1);
                } else if b1.0.starts_with("-") && b1.1.is_none() {
                    task_apply_tag_remove(t, ops, validation_result, b1);
                } else if b1.0.to_lowercase().as_str() == "depends" {
                    task_apply_depends(t, replica, ops, validation_result, b1);
                } else if b1.0.to_lowercase().trim() == "description" {
                    task_apply_description(t, ops, validation_result, b1);
                } else if b1.0.to_lowercase().trim() == "priority" {
                    task_apply_priority(t, ops, validation_result, b1);
                } else if ["entry", "wait", "due"].contains(&b1.0.to_lowercase().trim()) {
                    task_apply_timestamps(t, ops, validation_result, b1);
                } else if b1.0.to_lowercase().trim() == "status" {
                    task_apply_status(t, ops, validation_result, b1);
                } else if b1.0.to_lowercase().trim() == "recur" {
                    task_apply_recur(t, ops, validation_result, b1);
                } else if ["start", "stop", "done", "end", "modified"]
                    .contains(&b1.0.to_lowercase().trim())
                {
                    validation_result.push(FieldError {
                        field: "additional".into(),
                        message: format!(
                            "Manual modification of the field {} is not allowed.",
                            b1.0
                        ),
                    });
                } else if let Ok(_) = TaskProperties::try_from(b1.0.as_str()) {
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
        Err(e) => validation_result.push(e),
    }
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
        parse_apply_additions(
            &mut t,
            &mut replica,
            &mut ops,
            additional,
            &mut validation_result,
        );
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

/// Update a tasks with given information.
pub fn run_modify_command(
    uuid: Uuid,
    cmd_text: &str,
    app_state: &AppState,
) -> Result<(), FormValidation> {
    let mut validation_result = FormValidation::default();
    let mut replica = get_replica(&app_state.task_storage_path)
        .map_err(|err| <anyhow::Error as Into<FormValidation>>::into(err))?;
    let mut ops = Operations::new();
    ops.push(taskchampion::Operation::UndoPoint);

    let mut t = replica.get_task(uuid)?.expect("Valid task found");
    let old_task = t.clone();
    parse_apply_additions(
        &mut t,
        &mut replica,
        &mut ops,
        &cmd_text.to_string(),
        &mut validation_result,
    );

    match validation_result.is_success() {
        true => {
            // Commit those operations to storage.
            match replica.commit_operations(ops) {
                Ok(_) => {
                    info!("Updated task {}", uuid.to_string());
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
                        "Could not update task {}, error: {}",
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
pub fn list_tasks(
    task_query: &TaskQuery,
) -> Result<IndexMap<TaskUUID, crate::backend::task::Task>, anyhow::Error> {
    read_task_file(task_query)
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

    let mut t = replica.get_task(task.uuid)?.expect("Task does not exist");

    let old_task = t.clone();
    let task_status = convert_task_status(&task.status);

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
            let n = String::from_utf8(v.stdout)?;
            let res: Vec<Task> = serde_json::from_str(&n)?;
            if res.len() == 0 {
                Ok(None)
            } else {
                Ok(res.first().cloned())
            }
        }
    }
}

pub fn toggle_task_active(
    task_uuid: Uuid,
    task_status: String,
    app_state: &AppState,
) -> Result<bool, anyhow::Error> {
    let mut replica = get_replica(&app_state.task_storage_path)?;
    let mut ops = Operations::new();
    ops.push(taskchampion::Operation::UndoPoint);

    let mut t = replica
        .get_task(task_uuid)
        .unwrap()
        .expect("Task does not exist");

    let old_task = t.clone();
    let mut changed_tasks: Vec<(taskchampion::Task, taskchampion::Task)> = Vec::new();

    // Request to stop the job
    if task_status == "stop" {
        t.stop(&mut ops)?;
    }

    // Stop all active tasks.
    for mut single_task in replica.all_tasks()? {
        if single_task.1.is_active() {
            let old = single_task.1.clone();
            single_task.1.stop(&mut ops)?;
            let new = single_task.1.clone();
            changed_tasks.push((old, new));
        }
    }

    // Request to start the job
    if task_status == "start" {
        t.start(&mut ops)?;
    }
    changed_tasks.push((old_task, t.clone()));

    // Commit those operations to storage.
    match replica.commit_operations(ops) {
        Ok(_) => {
            info!("Task {} started", task_uuid.to_string());
            // execute hooks.
            for ct in changed_tasks {
                let _ = execute_hooks(
                    &app_state.task_hooks_path,
                    &TaskEvent::OnModify,
                    &Some(ct.0.into()),
                    &Some(ct.1.into()),
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

/// Read / Retrieve task by UUID
/// via task command line.
/// This is required required in order to get
/// priority information.
pub fn get_task_details(uuid: String) -> Result<crate::backend::task::Task, anyhow::Error> {
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

/// Update / Prepares a task detail page considering
/// Annotation sort and sort of task dependency list.
/// Annotations are sorted creation time descending (newest on top).
pub fn get_task_details_form(
    task: &mut crate::backend::task::Task,
    app_state: &AppState,
) -> Vec<crate::backend::task::Task> {
    // we must sort the annotations.
    if task.annotations.is_some() {
        task.annotations.as_mut().unwrap().sort();
        task.annotations.as_mut().unwrap().reverse();
    }
    // get dependent tasks if available.
    let mut tasks_deps: Vec<crate::backend::task::Task> = Vec::new();
    if let Some(dep_list) = &task.depends {
        for dep_uuid in dep_list {
            if let Ok(dep_task) = get_task(&app_state.task_storage_path, *dep_uuid) {
                if let Some(dep_task) = dep_task {
                    tasks_deps.push(dep_task)
                }
            }
        }
    }
    tasks_deps.sort_by(|a, b| {
        if a.status
            .clone()
            .unwrap_or(Status::Unknown("".to_string()))
            .to_string()
            == Status::Completed.to_string()
            && b.status
                .clone()
                .unwrap_or(Status::Unknown("".to_string()))
                .to_string()
                == Status::Completed.to_string()
        {
            Ordering::Equal
        } else if a
            .status
            .clone()
            .unwrap_or(Status::Unknown("".to_string()))
            .to_string()
            == Status::Completed.to_string()
            && b.status
                .clone()
                .unwrap_or(Status::Unknown("".to_string()))
                .to_string()
                != Status::Completed.to_string()
        {
            Ordering::Greater
        } else if a
            .status
            .clone()
            .unwrap_or(Status::Unknown("".to_string()))
            .to_string()
            != Status::Completed.to_string()
            && b.status
                .clone()
                .unwrap_or(Status::Unknown("".to_string()))
                .to_string()
                == Status::Completed.to_string()
        {
            Ordering::Less
        } else if a.id.unwrap_or(9999999) < b.id.unwrap_or(9999999) {
            Ordering::Less
        } else if a.id.unwrap_or(9999999) > b.id.unwrap_or(9999999) {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    });
    tasks_deps
}

/// Request to display a task detail page.
pub async fn display_task_details(
    Path(task_id): Path<Uuid>,
    app_state: State<AppState>,
) -> Response<String> {
    match get_task_details(task_id.to_string()) {
        Ok(mut task) => {
            let tasks_deps = get_task_details_form(&mut task, &app_state);
            let mut ctx: Context = get_default_context(&app_state);
            ctx.insert("tasks_db", &tasks_deps);
            let mut shortcuts: HashSet<String> = HashSet::new();
            let mut shortcut_list: Vec<String> = Vec::new();
            if let Some(anno_list) = &task.annotations {
                for _ in anno_list.iter().enumerate() {
                    let shortcut = make_shortcut(&mut shortcuts);
                    shortcut_list.push(shortcut);
                }
            }
            // annotate_shortcuts
            ctx.insert("annotate_shortcuts", &shortcut_list);
            ctx.insert("task", &task);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(TEMPLATES.render("task_details.html", &ctx).unwrap())
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("".to_string())
            .unwrap(),
    }
}

/// Request to get a confirmation screen prior
/// deleting a task.
pub async fn display_task_delete(
    Path(task_id): Path<Uuid>,
    app_state: State<AppState>,
) -> Response<String> {
    match get_task_details(task_id.to_string()) {
        Ok(mut task) => {
            let tasks_deps = get_task_details_form(&mut task, &app_state);
            let mut ctx: Context = get_default_context(&app_state);
            ctx.insert("tasks_db", &tasks_deps);
            // annotate_shortcuts
            ctx.insert("annotate_shortcuts", "");
            ctx.insert("task", &task);
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/html")
                .body(TEMPLATES.render("task_delete_confirm.html", &ctx).unwrap())
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("".to_string())
            .unwrap(),
    }
}

/// Process request to delete a specific annotation entry
/// from task.
pub async fn api_denotate_task_entry(
    Path(task_id): Path<Uuid>,
    app_state: State<AppState>,
    Form(denotate_form): Form<Annotation>,
) -> Response<String> {
    match get_task(&app_state.task_storage_path, task_id) {
        Ok(t) if t.is_some() => match denotate_task(task_id, &denotate_form, &app_state) {
            Ok(mut task) => {
                let tasks_deps = get_task_details_form(&mut task, &app_state);
                let mut ctx: Context = get_default_context(&app_state);
                ctx.insert("tasks_db", &tasks_deps);
                // annotate_shortcuts
                ctx.insert("annotate_shortcuts", "");
                ctx.insert("task", &task);
                Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "text/html")
                    .body(TEMPLATES.render("task_details.html", &ctx).unwrap())
                    .unwrap()
            }
            Err(_) => Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body("".to_string())
                .unwrap(),
        },
        Ok(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("".to_string())
            .unwrap(),
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("".to_string())
            .unwrap(),
    }
}

pub const TAG_KEYWORDS: [&str; 4] = ["next", "pending", "completed", "new"];

pub fn is_tag_keyword(tag: &str) -> bool {
    TAG_KEYWORDS.contains(&tag)
}

pub fn is_a_tag(tag: &str) -> bool {
    is_tag_keyword(tag) || tag.starts_with("+")
}

pub struct TaskViewDataRetType {
    pub tasks: IndexMap<TaskUUID, crate::backend::task::Task>,
    pub tag_map: HashMap<String, String>,
    pub shortcuts: HashSet<String>,
    pub task_list: Vec<crate::backend::task::Task>,
    pub task_shortcut_map: HashMap<String, String>,
    pub custom_queries_map: HashMap<String, CustomQuery>,
}

#[cfg(test)]
mod tests {
    use std::{env, str::FromStr};

    use chrono::{Datelike, Days, Month, Months, Timelike, Utc};
    use taskchampion::{Status, Tag, Uuid};
    use tempfile::{tempdir, TempDir};

    use crate::{
        backend::task::get_replica, core::app::AppState, endpoints::tasks::run_modify_command,
        NewTask,
    };

    use super::task_add;

    fn get_random_appstate() -> (TempDir, AppState) {
        let tmp_dir = tempdir().expect("Cannot create a tempdir.");
        env::set_var(
            "TASKDATA",
            tmp_dir.as_ref().to_str().expect("Cannot get path"),
        );
        let app_state = AppState::default();

        (tmp_dir, app_state)
    }

    #[test]
    fn test_task_add() {
        let (tmp_dir, app_state) = get_random_appstate();
        let task_name = Uuid::new_v4();

        let task = NewTask {
            description: task_name.clone().to_string(),
            tags: Some("+twk development".into()),
            project: Some("TWK".into()),
            filter_value: None,
            additional: Some("priority:H".into()),
        };
        let result = task_add(&task, &app_state);
        assert_eq!(result.is_ok(), true);
        let mut replica =
            get_replica(&app_state.task_storage_path).expect("Cannot retrieve replica");
        let tasks = replica.all_tasks().expect("Cannot retrieve tasks");
        let our_task = tasks
            .iter()
            .find(|p| p.1.get_description() == &task_name.to_string());
        assert_eq!(our_task.is_some(), true);
        let our_task = our_task.expect("Cannot unwrap task");
        // compare the data.
        let task_map = our_task.1.clone().into_task_data();
        assert_eq!(task_map.get("project"), Some("TWK"));
        let tags: Vec<Tag> = our_task.1.get_tags().collect();
        assert_eq!(tags.contains(&Tag::from_str("twk").unwrap()), true);
        assert_eq!(tags.contains(&Tag::from_str("development").unwrap()), true);
        assert_eq!(task_map.get("priority"), Some("H"));

        let _ = tmp_dir.close();
    }

    #[test]
    fn test_task_add_fail() {
        let (tmp_dir, app_state) = get_random_appstate();
        let task_name = Uuid::new_v4();

        let task = NewTask {
            description: task_name.clone().to_string(),
            tags: Some("+twk development".into()),
            project: Some("TWK".into()),
            filter_value: None,
            additional: Some("priority:H due:\"".into()),
        };
        let result = task_add(&task, &app_state);
        assert_eq!(result.is_ok(), false);
        let result = result.unwrap_err();
        assert_eq!(result.is_success(), false);
        assert_eq!(result.has_error("additional"), true);

        let _ = tmp_dir.close();
    }

    #[test]
    fn test_task_modify_successful() {
        let (tmp_dir, app_state) = get_random_appstate();
        let task_name = Uuid::new_v4();

        let task = NewTask {
            description: task_name.clone().to_string(),
            tags: Some("+twk development".into()),
            project: Some("TWK".into()),
            filter_value: None,
            additional: Some("priority:H".into()),
        };
        let result = task_add(&task, &app_state);
        assert_eq!(result.is_ok(), true);
        let mut replica =
            get_replica(&app_state.task_storage_path).expect("Cannot retrieve replica");
        let tasks = replica.all_tasks().expect("Cannot retrieve tasks");
        let our_task_1 = tasks
            .iter()
            .find(|p| p.1.get_description() == &task_name.to_string());
        assert_eq!(our_task_1.is_some(), true);
        let our_task_1 = our_task_1.expect("Cannot unwrap task");

        // create a second one.
        let task_name_2 = Uuid::new_v4();

        let task = NewTask {
            description: task_name_2.clone().to_string(),
            tags: Some("+twk development".into()),
            project: Some("TWK".into()),
            filter_value: None,
            additional: Some("priority:H".into()),
        };
        let result = task_add(&task, &app_state);
        assert_eq!(result.is_ok(), true);
        let tasks = replica.all_tasks().expect("Cannot retrieve tasks");
        let our_task_2 = tasks
            .iter()
            .find(|p| p.1.get_description() == &task_name_2.to_string());
        assert_eq!(our_task_2.is_some(), true);

        let dt_wait = Utc::now().checked_add_days(Days::new(15)).unwrap();
        let dt_wait_str = dt_wait.format("%Y-%m-%d").to_string();
        let dt_wait = dt_wait
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();
        let dt_due = Utc::now().checked_add_days(Days::new(60)).unwrap();
        let dt_due_str = dt_due.format("%Y-%m-%d").to_string();
        let dt_due = dt_due
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap()
            .with_nanosecond(0)
            .unwrap();

        // Now modify!
        let cmd_text = format!(
            "wait:{} due:{} +concert -twk \"description:This is a title with spaces\" depends:{} project:{}  status:completed",
            dt_wait_str,
            dt_due_str,
            task_name_2.to_string(),
            "KWT"
        );
        let result = run_modify_command(our_task_1.0.clone(), &cmd_text, &app_state);
        assert_eq!(result.is_ok(), true);
        let updated_task = replica.get_task(our_task_1.0.clone());
        assert_eq!(updated_task.is_ok(), true);
        let updated_task = updated_task.unwrap();
        assert_eq!(updated_task.is_some(), true);
        let updated_task = updated_task.unwrap();

        // Compare the data.
        assert_eq!(
            updated_task.get_description(),
            "This is a title with spaces"
        );
        assert_eq!(updated_task.get_value("project"), Some("KWT"));
        assert_eq!(updated_task.get_due(), Some(dt_due));
        assert_eq!(updated_task.get_wait(), Some(dt_wait));
        assert_eq!(updated_task.is_waiting(), true);
        assert_eq!(updated_task.get_dependencies().count(), 1);
        assert_eq!(updated_task.get_status(), Status::Completed);
        assert_eq!(updated_task.get_dependencies().next(), Some(task_name_2));
        let set_tags = vec![
            "concert",
            "development",
            "WAITING",
            "COMPLETED",
            "UNBLOCKED",
        ];
        for tag in updated_task.get_tags() {
            let tag_name = tag.to_string();
            println!("Comparing {} in {:?}", tag_name, set_tags);
            assert_eq!(set_tags.contains(&tag_name.as_str()), true);
        }

        let _ = tmp_dir.close();
    }

    #[test]
    fn test_task_modify_breakit() {
        let (tmp_dir, app_state) = get_random_appstate();
        let task_name = Uuid::new_v4();

        let task = NewTask {
            description: task_name.clone().to_string(),
            tags: Some("+twk development".into()),
            project: Some("TWK".into()),
            filter_value: None,
            additional: Some("priority:H".into()),
        };
        let result = task_add(&task, &app_state);
        assert_eq!(result.is_ok(), true);
        let mut replica =
            get_replica(&app_state.task_storage_path).expect("Cannot retrieve replica");
        let tasks = replica.all_tasks().expect("Cannot retrieve tasks");
        let our_task_1 = tasks
            .iter()
            .find(|p| p.1.get_description() == &task_name.to_string());
        assert_eq!(our_task_1.is_some(), true);
        let our_task_1 = our_task_1.expect("Cannot unwrap task");

        // Now modify!
        let cmd_text = format!(
            "wait:{} due:{} +concert -twk +d/c -e/b +WAITING -PENDING start \"description:This is a title with spaces\" depends:{} project:{}",
            "abc",
            "def",
            String::from("ec2c596f-5fa3-442c-80ee-98b087e32bbd"),
            ""
        );
        let result = run_modify_command(our_task_1.0.clone(), &cmd_text, &app_state);

        let result = result.unwrap_err();
        println!("{:?}", result);

        assert_eq!(result.is_success(), false);
        assert_eq!(result.has_error("additional"), true);
        let add_errors = result.fields.get("additional");
        assert_eq!(add_errors.is_some(), true);
        let add_errors = add_errors.unwrap();
        assert_eq!(add_errors.len(), 7);

        let mut keywords = vec![
            "wait",
            "due",
            "start",
            "tag",
            "tag",
            "Synthetic",
            "Synthetic",
        ];
        for err in add_errors {
            assert_eq!(&err.field, "additional");
            let p = keywords.iter().position(|p| err.message.contains(*p));
            assert_eq!(p.is_some(), true);
            let p = p.unwrap();
            let _ = keywords.remove(p);
        }
        assert_eq!(keywords.is_empty(), true);

        let _ = tmp_dir.close();
    }

    #[test]
    fn test_task_add_recur() {
        let (tmp_dir, app_state) = get_random_appstate();
        let task_name = Uuid::new_v4();
        let dt_wait = Utc::now()
            .with_day(1)
            .unwrap()
            .checked_add_months(Months::new(1))
            .unwrap();

        let task = NewTask {
            description: task_name.clone().to_string(),
            tags: Some("+twk development".into()),
            project: Some("TWK".into()),
            filter_value: None,
            additional: Some(format!(
                "priority:H recur:monthly due:{}",
                dt_wait.format("%Y-%m-%d")
            )),
        };
        let result = task_add(&task, &app_state);
        assert_eq!(result.is_ok(), true);
        let mut replica =
            get_replica(&app_state.task_storage_path).expect("Cannot retrieve replica");
        let tasks = replica.all_tasks().expect("Cannot retrieve tasks");
        let our_task = tasks
            .iter()
            .find(|p| p.1.get_description() == &task_name.to_string());
        assert_eq!(our_task.is_some(), true);
        let our_task = our_task.expect("Cannot unwrap task");
        // compare the data.
        assert_eq!(our_task.1.get_status(), Status::Recurring);
        assert_eq!(our_task.1.get_value("rtype"), Some("periodic"));
        assert_eq!(our_task.1.get_value("recur"), Some("monthly"));

        let _ = tmp_dir.close();
    }
}
