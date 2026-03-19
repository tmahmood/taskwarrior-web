/*
 * Copyright 2025 Tarin Mahmood
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
 */
use std::str::FromStr;

use crate::{
    backend::task::convert_task_status,
    core::errors::{FieldError, FormValidation},
};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use taskchampion::storage::Storage;
use taskchampion::{Replica, Tag, Uuid};

fn get_tag_from_str(
    maybe_tag_name: &str,
    strip: &str,
    validation_result: &mut FormValidation,
) -> Option<Tag> {
    let tag_name = maybe_tag_name.strip_prefix(strip).unwrap_or(maybe_tag_name);
    match Tag::from_str(tag_name) {
        Ok(tag) => Some(tag),
        Err(e) => {
            validation_result.push(FieldError {
                field: "additional".to_string(),
                message: format!("{e:?}"),
            });
            None
        }
    }
}

pub fn task_apply_tag_add(
    task: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: &(String, Option<String>),
) {
    let maybe_tag = get_tag_from_str(&b1.0, "+", validation_result);
    let Some(tag) = maybe_tag else { return };
    if let Err(err) = task.add_tag(&tag, ops) {
        validation_result.push(FieldError {
            field: "additional".to_string(),
            message: err.to_string(),
        });
    }
}

pub fn task_apply_tag_remove(
    task: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: &(String, Option<String>),
) {
    let maybe_tag = get_tag_from_str(&b1.0, "-", validation_result);
    let Some(tag) = maybe_tag else { return };
    if let Err(err) = task.remove_tag(&tag, ops) {
        validation_result.push(FieldError {
            field: "additional".to_string(),
            message: err.to_string(),
        });
    }
}

pub fn task_apply_recur(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    match t
        .set_value("recur", b1.1, ops)
        .map_err(|p| FieldError {
            field: "additional".to_string(),
            message: format!("Failed change recurrence: {p}"),
        })
        .and_then(|()| {
            t.set_status(taskchampion::Status::Recurring, ops)
                .map_err(|p| FieldError {
                    field: "additional".to_string(),
                    message: format!("Failed change task status to recurring: {p}"),
                })
                .and_then(|()| {
                    t.set_value("rtype", Some("periodic".into()), ops)
                        .map_err(|p| FieldError {
                            field: "additional".to_string(),
                            message: format!("Failed change task status to recurring: {p}"),
                        })
                })
        }) {
        Ok(()) => (),
        Err(e) => validation_result.push(e),
    }
}

pub async fn task_apply_depends<S: Storage>(
    t: &mut taskchampion::Task,
    replica: &mut Replica<S>,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    let dep_list = b1.1.unwrap_or_default();
    for dep in dep_list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let result = match dep.chars().next() {
            Some(e) if (e == '+' || e == '-') && dep.len() > 1 => Some((e, dep.get(1..).unwrap())),
            Some(_) if !dep.is_empty() => {
                // We assume adding.
                Some(('+', dep))
            }
            Some(_) | None => None,
        };
        if let Some(result) = result {
            // Try to identify the uuid.
            let found_uuid = if let Ok(e) = Uuid::try_parse(result.1) {
                Some(e)
            } else {
                match result.1.parse::<usize>() {
                    Ok(e) => replica.working_set().await.unwrap().by_index(e),
                    Err(_) => None,
                }
            };
            if let Some(task_uuid) = found_uuid {
                let dep_result = match result.0 {
                    '-' => t.remove_dependency(task_uuid, ops),
                    _ => t.add_dependency(task_uuid, ops),
                };
                match dep_result.map_err(|p| FieldError {
                    field: "additional".to_string(),
                    message: format!("depends-error for uuid {task_uuid}: {p}"),
                }) {
                    Ok(()) => (),
                    Err(e) => validation_result.push(e),
                }
            } else {
                validation_result.push(FieldError {
                    field: String::from("additional"),
                    message: format!(
                        "Dependency task {} not found or invalid ID given.",
                        result.1
                    ),
                });
            }
        }
    }
}

pub fn task_apply_description(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    match t
        .set_description(b1.1.unwrap_or_default(), ops)
        .map_err(|p| FieldError {
            field: "additional".to_string(),
            message: format!("Invalid description given: {p}"),
        }) {
        Ok(()) => (),
        Err(e) => validation_result.push(e),
    }
}

pub fn task_apply_priority(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    match t
        .set_priority(b1.1.unwrap_or_default(), ops)
        .map_err(|p| FieldError {
            field: "additional".to_string(),
            message: format!("Invalid priority given: {p}"),
        }) {
        Ok(()) => (),
        Err(e) => validation_result.push(e),
    }
}

pub fn task_apply_timestamps(
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
                message: format!("Failed parsing timestamp for {} ({}).", &b1.0, p),
            })
            .map(Some),
        Some(_) | None => Ok(None),
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
                message: format!("Failed setting timestamp for {} ({}).", &b1.0, p),
            });
            if let Err(p) = result {
                validation_result.push(p);
            }
        }
        Err(e) => validation_result.push(e),
    }
}

pub fn task_apply_status(
    t: &mut taskchampion::Task,
    ops: &mut Vec<taskchampion::Operation>,
    validation_result: &mut FormValidation,
    b1: (String, Option<String>),
) {
    if let Some(val) = b1.1 {
        let task_status = convert_task_status(&val);
        match t.set_status(task_status, ops).map_err(|p| FieldError {
            field: "additional".into(),
            message: format!("Invalid status {} ({}).", &val, p),
        }) {
            Ok(()) => (),
            Err(p) => validation_result.push(p),
        }
    }
}
