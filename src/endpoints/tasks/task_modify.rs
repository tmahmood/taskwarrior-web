use std::str::FromStr;

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use taskchampion::{Replica, Tag, Uuid};

use crate::{
    backend::task::convert_task_status,
    core::errors::{FieldError, FormValidation},
};

pub(crate) fn task_apply_tag_add(
    t: &mut taskchampion::Task,
    mut ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    let tag_name = b1.0.strip_prefix("+").unwrap();
    match &Tag::from_str(tag_name).map_err(|p| FieldError {
        field: "additional".to_string(),
        message: p.to_string(),
    }) {
        Ok(tag) => match t.add_tag(tag, &mut ops).map_err(|p| FieldError {
            field: "additional".to_string(),
            message: p.to_string(),
        }) {
            Ok(_) => (),
            Err(e) => validation_result.push(e),
        },
        Err(e) => validation_result.push(e.to_owned()),
    };
}

pub(crate) fn task_apply_tag_remove(
    t: &mut taskchampion::Task,
    mut ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    let tag_name = b1.0.strip_prefix("-").unwrap();
    match &Tag::from_str(tag_name).map_err(|p| FieldError {
        field: "additional".to_string(),
        message: p.to_string(),
    }) {
        Ok(tag) => match t.remove_tag(tag, &mut ops).map_err(|p| FieldError {
            field: "additional".to_string(),
            message: p.to_string(),
        }) {
            Ok(_) => (),
            Err(e) => validation_result.push(e),
        },
        Err(e) => validation_result.push(e.to_owned()),
    };
}

pub(crate) fn task_apply_recur(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    match t
        .set_value("recur", b1.1, ops)
        .map_err(|p| FieldError {
            field: "additional".to_string(),
            message: format!("Failed change recurrence: {}", p.to_string()),
        })
        .and_then(|_| {
            t.set_status(taskchampion::Status::Recurring, ops)
                .map_err(|p| FieldError {
                    field: "additional".to_string(),
                    message: format!("Failed change task status to recurring: {}", p.to_string()),
                })
                .and_then(|_| {
                    t.set_value("rtype", Some("periodic".into()), ops)
                        .map_err(|p| FieldError {
                            field: "additional".to_string(),
                            message: format!(
                                "Failed change task status to recurring: {}",
                                p.to_string()
                            ),
                        })
                })
        }) {
        Ok(_) => (),
        Err(e) => validation_result.push(e),
    };
}

pub(crate) fn task_apply_depends(
    t: &mut taskchampion::Task,
    replica: &mut Replica,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    let dep_list = b1.1.unwrap_or_default();
    for dep in dep_list
        .split(",")
        .map(|f| f.trim())
        .filter(|p| !p.is_empty())
    {
        let result = match dep.chars().next() {
            Some(e) if (e == '+' || e == '-') && dep.len() > 1 => Some((e, dep.get(1..).unwrap())),
            Some(_) if !dep.is_empty() => {
                // We assume adding.
                Some(('+', dep))
            }
            Some(_) => None,
            None => None,
        };
        if let Some(result) = result {
            // Try to identify the uuid.
            let x = Uuid::try_parse(result.1);
            let x = match x {
                Ok(e) => Some(e),
                Err(_) => {
                    let tid = result.1.parse::<usize>();
                    match tid {
                        Ok(e) => replica.working_set().unwrap().by_index(e),
                        Err(_) => None,
                    }
                }
            };
            if let Some(task_uuid) = x {
                let dep_result = match result.0 {
                    '-' => t.remove_dependency(task_uuid, ops),
                    _ => t.add_dependency(task_uuid, ops),
                };
                match dep_result.map_err(|p| FieldError {
                    field: "additional".to_string(),
                    message: format!(
                        "depends-error for uuid {}: {}",
                        task_uuid.to_string(),
                        p.to_string()
                    ),
                }) {
                    Ok(_) => (),
                    Err(e) => validation_result.push(e),
                };
            } else {
                validation_result.push(FieldError {
                    field: String::from("additional"),
                    message: format!(
                        "Dependency task {} not found or invalid ID given.",
                        result.1
                    ),
                });
            }
        };
    }
}

pub(crate) fn task_apply_description(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    match t
        .set_description(b1.1.unwrap_or_default(), ops)
        .map_err(|p| FieldError {
            field: "additional".to_string(),
            message: format!("Invalid description given: {}", p.to_string()),
        }) {
        Ok(_) => (),
        Err(e) => validation_result.push(e),
    };
}

pub(crate) fn task_apply_priority(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    match t
        .set_priority(b1.1.unwrap_or_default(), ops)
        .map_err(|p| FieldError {
            field: "additional".to_string(),
            message: format!("Invalid priority given: {}", p.to_string()),
        }) {
        Ok(_) => (),
        Err(e) => validation_result.push(e),
    };
}

pub(crate) fn task_apply_timestamps(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    let dt = match b1.1 {
        Some(val) if !val.trim().is_empty() => val
            .trim()
            .parse::<DateTime<Utc>>()
            .or_else(|_| {
                val.parse::<NaiveDate>().map(|p| {
                    p.and_time(
                        NaiveTime::from_num_seconds_from_midnight_opt(0, 0)
                            .expect("Failed even to create the simplest Time object"),
                    )
                    .and_utc()
                })
            })
            .map_err(|p| FieldError {
                field: "additional".into(),
                message: format!(
                    "Failed parsing timestamp for {} ({}).",
                    &b1.0,
                    p.to_string()
                ),
            })
            .map(|p| Some(p)),
        Some(_) => Ok(None),
        None => Ok(None),
    };
    match dt {
        Ok(e) => {
            let result = match b1.0.to_lowercase().trim() {
                "entry" => t.set_entry(e, ops),
                "wait" => t.set_wait(e, ops),
                "due" => t.set_due(e, ops),
                _ => Ok(()),
            }
            .map_err(|p| FieldError {
                field: "additional".into(),
                message: format!(
                    "Failed setting timestamp for {} ({}).",
                    &b1.0,
                    p.to_string()
                ),
            });
            if let Err(p) = result {
                validation_result.push(p);
            }
        }
        Err(e) => validation_result.push(e),
    };
}

pub(crate) fn task_apply_status(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    if let Some(val) = b1.1 {
        let task_status = convert_task_status(&val);
        match t.set_status(task_status, ops).map_err(|p| FieldError {
            field: "additional".into(),
            message: format!("Invalid status {} ({}).", &val, p.to_string()),
        }) {
            Ok(_) => (),
            Err(p) => validation_result.push(p),
        };
    }
}
