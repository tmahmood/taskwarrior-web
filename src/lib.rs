#![feature(exit_status_error)]
#![feature(let_chains)]

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{de, Deserialize, Deserializer};
use std::str::FromStr;
use std::fmt;
use serde::de::Error;
use tracing::trace;
use std::collections::HashMap;
use chrono::Local;
use crate::endpoints::tasks::Task;

lazy_static::lazy_static! {
    pub static ref TEMPLATES: tera::Tera = {
        let mut tera = match tera::Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.register_function("project_name", get_project_name_link());
        tera.register_function("date_proper", get_date_proper());
        tera.autoescape_on(vec![
            ".html",
            ".sql"
        ]);
        tera
    };
}

pub struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
    where
        E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub mod endpoints;


#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Params {
    query: Option<String>,
    q: Option<String>,
    status: Option<String>,
    uuid: Option<String>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            query: Some("status:pending".to_string()),
            q: None,
            status: None,
            uuid: None,
        }
    }
}

pub struct TaskUpdateStatus {
    pub status: String,
    pub uuid: String,
}

impl Params {
    pub fn task(&self) -> Option<TaskUpdateStatus> {
        if let Some(uuid) = self.uuid.as_ref() && let Some(status) = self.status.as_ref() {
            return Some(TaskUpdateStatus {
                status: status.clone(),
                uuid: uuid.clone(),
            });
        }
        None
    }

    pub fn query(&self) -> Vec<&str> {
        trace!("{:?} {:?}", self.query, self.q);
        let mut current_filters = if let Some(tlist) = self.query.as_ref() {
            if tlist == "[ALL]" {
                vec![]
            } else {
                tlist.trim()
                    .split(" ")
                    .filter(|v| *v != " " || *v != "")
                    .map(|v| v.trim())
                    .collect()
            }
        } else {
            vec![]
        };
        let q = self.q.as_ref();
        if let Some(_q) = q {
            if _q.starts_with("priority=") {
                current_filters.retain_mut(|iv| !iv.starts_with("priority="));
                current_filters.push(_q);
            } else if _q.starts_with("status:") {
                current_filters.retain_mut(|iv| !iv.starts_with("status:"));
                current_filters.push(_q);
            } else if current_filters.contains(&_q.as_str()) {
                current_filters.retain_mut(|iv| iv != &_q);
            } else {
                current_filters.push(_q);
            }
        }
        current_filters
    }
}

/// Serde deserialization decorator to map empty Strings to None,
pub fn empty_string_as_none<'de, D, T>(de: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: FromStr,
        T::Err: fmt::Display,
{
    let opt = Option::<String>::deserialize(de)?;
    match opt.as_deref() {
        None | Some("") => Ok(None),
        Some(s) => FromStr::from_str(s).map_err(de::Error::custom).map(Some),
    }
}

fn get_project_name_link() -> impl tera::Function {
    Box::new(move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
        let r = String::new();
        let pname = tera::from_value::<String>(
            args.get("full_name").clone().unwrap().clone()
        ).unwrap();
        let index = tera::from_value::<usize>(
            args.get("index").clone().unwrap().clone()
        ).unwrap();
        let r: Vec<&str> = pname.split(".").take(index).collect();
        Ok(tera::to_value(r.join(".")).unwrap())
    })
}

fn get_date_proper() -> impl tera::Function {
    Box::new(move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
        let time = chrono::prelude::NaiveDateTime::parse_from_str(
            args.get("date").unwrap().as_str().unwrap(),
            "%Y%m%dT%H%M%SZ"
        ).unwrap().and_local_timezone(Local).unwrap();
        let s = time.format("%Y-%m-%d %H:%M:%S").to_string();
        Ok(tera::to_value(s).unwrap())
    })
}