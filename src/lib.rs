#![feature(exit_status_error)]
#![feature(let_chains)]

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde::de::Error;
use tracing::info;

use crate::endpoints::tasks::task_query_builder::{TaskQuery, TaskReport};

lazy_static::lazy_static! {
    pub static ref TEMPLATES: tera::Tera = {
        let mut tera = match tera::Tera::new("frontend/templates/**/*") {
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

pub enum Requests {
    Filtering {
        project: Option<String>,
        tags: Option<Vec<String>>,
    }
}


#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FlashMsg {
    msg: String,
    timeout: Option<u64>,
}

impl FlashMsg {
    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn timeout(&self) -> u64 {
        self.timeout.clone().unwrap_or(15)
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TWGlobalState {
    filter: Option<String>,
    query: Option<String>,
    report: Option<String>,
    status: Option<String>,
    uuid: Option<String>,
    filter_value: Option<String>,
    task_entry: Option<String>,
}

impl TWGlobalState {
    pub fn filter(&self) -> &Option<String> {
        &self.filter
    }
    pub fn query(&self) -> &Option<String> {
        &self.query
    }
    pub fn report(&self) -> &Option<String> {
        &self.report
    }
    pub fn status(&self) -> &Option<String> {
        &self.status
    }
    pub fn uuid(&self) -> &Option<String> {
        &self.uuid
    }
    pub fn filter_value(&self) -> &Option<String> {
        &self.filter_value
    }
    pub fn task_entry(&self) -> &Option<String> {
        &self.task_entry
    }
}


pub fn task_query_merge_previous_params(state: &TWGlobalState) -> TaskQuery {
    if let Some(fv) = state.filter_value.clone() {
        let mut tq: TaskQuery = serde_json::from_str(&fv).unwrap();
        tq.update(state.clone());
        tq
    } else {
        TaskQuery::new(TWGlobalState::default())
    }
}

pub fn task_query_previous_params(params: &TWGlobalState) -> TaskQuery {
    if let Some(fv) = params.filter_value.clone() {
        serde_json::from_str(&fv).unwrap()
    } else {
        TaskQuery::new(TWGlobalState::default())
    }
}

pub fn from_task_to_task_update(params: &TWGlobalState) -> Option<TaskUpdateStatus> {
    if let Some(uuid) = params.uuid.as_ref() && let Some(status) = params.status.as_ref() {
        return Some(TaskUpdateStatus {
            status: status.clone(),
            uuid: uuid.clone(),
        });
    }
    None
}


impl Default for TWGlobalState {
    fn default() -> Self {
        Self {
            filter: None,
            query: None,
            report: Some(TaskReport::Next.to_string()),
            status: None,
            uuid: None,
            filter_value: None,
            task_entry: None,
        }
    }
}

pub struct TaskUpdateStatus {
    pub status: String,
    pub uuid: String,
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
        // we are working with utc time
        let time = chrono::prelude::NaiveDateTime::parse_from_str(
            args.get("date").unwrap().as_str().unwrap(),
            "%Y%m%dT%H%M%SZ",
        ).unwrap().and_utc();

        let in_future = args.get("in_future")
            .cloned()
            .unwrap_or(tera::Value::Bool(false))
            .as_bool().unwrap();

        let now = chrono::prelude::Utc::now();

        let delta = now - time;
        let num_weeks = delta.num_weeks();
        let num_days = delta.num_days();
        let num_hours = delta.num_hours();
        let num_minutes = delta.num_minutes();

        let sign = if in_future { -1 } else { 1 };

        let mut s = if num_weeks.abs() > 0 {
            format!("{}w", sign * num_weeks)
        } else if num_days.abs() > 0 {
            format!("{}d", sign * num_days)
        } else if num_hours.abs() > 0 {
            format!("{}h", sign * num_hours)
        } else {
            format!("{}m", sign * num_minutes)
        };
        Ok(tera::to_value(s).unwrap())
    })
}