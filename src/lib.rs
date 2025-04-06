#![feature(exit_status_error)]
#![feature(let_chains)]

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

use crate::endpoints::tasks::task_query_builder::{TaskQuery, TaskReport};
use crate::endpoints::tasks::{is_a_tag, is_tag_keyword};
use chrono::{DateTime, TimeDelta};
use rand::distr::{Alphanumeric, SampleString};
use serde::{de, Deserialize, Deserializer, Serialize};
use taskchampion::Uuid;
use tera::Context;
use tracing::warn;

lazy_static::lazy_static! {
    pub static ref TEMPLATES: tera::Tera = {
        let mut tera = match tera::Tera::new("dist/templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                warn!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        };
        tera.register_function("project_name", get_project_name_link());
        tera.register_function("date_proper", get_date_proper());
        tera.register_function("timer_value", get_timer());
        tera.register_function("date", get_date());
        tera.register_function("obj", obj());
        tera.register_function("remove_project_tag", remove_project_from_tag());
        tera.register_filter("update_unique_tags", update_unique_tags());
        tera.register_filter("update_tag_bar_key_comb", update_tag_bar_key_comb());
        tera.register_tester("keyword_tag", is_tag_keyword_tests());
        tera.register_tester("user_tag", is_tag_tests());
        tera.autoescape_on(vec![
            ".html",
            ".sql"
        ]);
        tera
    };
}

pub mod backend;
pub mod core;
pub mod endpoints;

pub enum Requests {
    Filtering {
        project: Option<String>,
        tags: Option<Vec<String>>,
    },
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FlashMsgRoles {
    Success,
    Error,
    Warning,
    Info,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FlashMsg {
    msg: String,
    timeout: Option<u64>,
    role: FlashMsgRoles,
}

impl FlashMsg {
    pub fn msg(&self) -> &str {
        &self.msg
    }

    pub fn role(&self) -> &FlashMsgRoles {
        &self.role
    }

    pub fn timeout(&self) -> u64 {
        self.timeout.clone().unwrap_or(15)
    }

    pub fn new(msg: &str, timeout: Option<u64>, role: FlashMsgRoles) -> Self {
        Self {
            msg: msg.to_string(),
            timeout,
            role: role
        }
    }

    pub fn to_context(&self, ctx: &mut Context) {
        ctx.insert("has_toast", &true);
        ctx.insert("toast_msg", &self.msg());
        ctx.insert("toast_role", &self.role());
        ctx.insert("toast_timeout", &self.timeout());
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub enum TaskActions {
    StatusUpdate,
    ToggleTimer,
    ModifyTask,
    AnnotateTask,
    DenotateTask,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TWGlobalState {
    filter: Option<String>,
    query: Option<String>,
    report: Option<String>,
    status: Option<String>,
    uuid: Option<Uuid>,
    filter_value: Option<String>,
    task_entry: Option<String>,
    action: Option<TaskActions>,
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
    pub fn uuid(&self) -> &Option<Uuid> {
        &self.uuid
    }
    pub fn filter_value(&self) -> &Option<String> {
        &self.filter_value
    }
    pub fn task_entry(&self) -> &Option<String> {
        &self.task_entry
    }
    pub fn action(&self) -> &Option<TaskActions> {
        &self.action
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
    if let Some(uuid) = params.uuid.as_ref()
        && let Some(status) = params.status.as_ref()
    {
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
            action: None,
        }
    }
}

#[derive(Clone)]
pub struct TaskUpdateStatus {
    pub status: String,
    pub uuid: Uuid,
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

fn remove_project_from_tag() -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let mut pname =
                tera::from_value::<String>(args.get("task").clone().unwrap().clone()).unwrap();
            pname = pname
                .replace("project:", "")
                .split(".")
                .last()
                .unwrap()
                .to_string();
            Ok(tera::to_value(pname).unwrap())
        },
    )
}

fn get_project_name_link() -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let pname =
                tera::from_value::<String>(args.get("full_name").clone().unwrap().clone()).unwrap();
            let index =
                tera::from_value::<usize>(args.get("index").clone().unwrap().clone()).unwrap();
            let r: Vec<&str> = pname.split(".").take(index).collect();
            Ok(tera::to_value(r.join(".")).unwrap())
        },
    )
}

fn is_tag_keyword_tests() -> impl tera::Test {
    Box::new(
        move |val: Option<&tera::Value>, _values: &[tera::Value]| -> tera::Result<bool> {
            let v_str = val.as_ref().unwrap().to_string();
            Ok(is_tag_keyword(&v_str))
        },
    )
}

fn is_tag_tests() -> impl tera::Test {
    Box::new(
        move |val: Option<&tera::Value>, _values: &[tera::Value]| -> tera::Result<bool> {
            let v_str = val.as_ref().unwrap().to_string();
            Ok(is_a_tag(&v_str))
        },
    )
}

fn update_unique_tags() -> impl tera::Filter {
    Box::new(
        move |value: &tera::Value,
              args: &HashMap<String, tera::Value>|
              -> tera::Result<tera::Value> {
            let mut tags = tera::from_value::<Vec<String>>(value.clone())?;
            let new_tag = tera::from_value::<String>(args.get("tag").clone().unwrap().clone())?;
            tags.push(new_tag);
            Ok(tera::to_value(tags)?)
        },
    )
}

fn obj() -> impl tera::Function {
    Box::new(
        move |_args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            let hm: HashMap<String, String> = HashMap::new();
            Ok(tera::to_value(hm)?)
        },
    )
}

fn update_tag_bar_key_comb() -> impl tera::Filter {
    Box::new(
        move |value: &tera::Value,
              args: &HashMap<String, tera::Value>|
              -> tera::Result<tera::Value> {
            let mut tag_key_comb = tera::from_value::<HashMap<String, String>>(value.clone())?;
            let tag = tera::from_value::<String>(args.get("tag").clone().unwrap().clone())?;
            loop {
                let string = Alphanumeric
                    .sample_string(&mut rand::rng(), 2)
                    .to_lowercase();
                if tag_key_comb.iter().find(|&(_k, v)| v == &string).is_some() {
                    continue;
                }
                tag_key_comb.insert(tag, string);
                break;
            }
            Ok(tera::to_value(tag_key_comb)?)
        },
    )
}

pub struct DeltaNow {
    pub now: DateTime<chrono::Utc>,
    pub delta: TimeDelta,
    pub time: DateTime<chrono::Utc>,
}

impl DeltaNow {
    pub fn new(time: &str) -> Self {
        let time = chrono::prelude::NaiveDateTime::parse_from_str(time, "%Y%m%dT%H%M%SZ")
            .unwrap_or_else(|_|
                // Try taskchampions variant.
                chrono::prelude::NaiveDateTime::parse_from_str(time, "%Y-%m-%dT%H:%M:%SZ").unwrap()
            )
            .and_utc();
        let now = chrono::prelude::Utc::now();
        let delta = now - time;
        Self { now, delta, time }
    }
}

fn get_date_proper() -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            // we are working with utc time
            let DeltaNow {
                now: _,
                delta,
                time: _time,
            } = DeltaNow::new(args.get("date").unwrap().as_str().unwrap());

            let num_weeks = delta.num_weeks();
            let num_days = delta.num_days();
            let num_hours = delta.num_hours();
            let num_minutes = delta.num_minutes();

            let in_future = args
                .get("in_future")
                .cloned()
                .unwrap_or(tera::Value::Bool(false))
                .as_bool()
                .unwrap();

            let sign = if in_future { -1 } else { 1 };

            let s = if num_weeks.abs() > 0 {
                format!("{}w", sign * num_weeks)
            } else if num_days.abs() > 0 {
                format!("{}d", sign * num_days)
            } else if num_hours.abs() > 0 {
                format!("{}h", sign * num_hours)
            } else {
                format!("{}m", sign * num_minutes)
            };
            Ok(tera::to_value(s).unwrap())
        },
    )
}

fn get_date() -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            // we are working with utc time
            let DeltaNow { time, .. } = DeltaNow::new(args.get("date").unwrap().as_str().unwrap());
            Ok(tera::to_value(time.format("%Y-%m-%d %H:%MZ").to_string()).unwrap())
        },
    )
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct NewTask {
    description: String,
    tags: Option<String>,
    project: Option<String>,
    filter_value: Option<String>,
    additional: Option<String>,
}

impl NewTask {
    pub fn new(
        description: Option<String>,
        tags: Option<String>,
        project: Option<String>,
        filter_value: Option<String>,
        additional: Option<String>,
    ) -> Self {
        Self {
            description: description.unwrap_or_default(),
            tags,
            project,
            filter_value,
            additional,
        }
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn tags(&self) -> &Option<String> {
        &self.tags
    }
    pub fn project(&self) -> &Option<String> {
        &self.project
    }
    pub fn filter_value(&self) -> &Option<String> {
        &self.filter_value
    }
    pub fn additional(&self) -> &Option<String> {
        &self.additional
    }
}

fn get_timer() -> impl tera::Function {
    Box::new(
        move |args: &HashMap<String, tera::Value>| -> tera::Result<tera::Value> {
            // we are working with utc time
            let DeltaNow { delta, .. } = DeltaNow::new(args.get("date").unwrap().as_str().unwrap());
            let num_seconds = delta.num_seconds();

            let s = if delta.num_hours() > 0 {
                format!(
                    "{:>02}:{:>02}",
                    delta.num_hours(),
                    delta.num_minutes() - (delta.num_hours() * 60)
                )
            } else if delta.num_minutes() > 0 {
                format!(
                    "{:>02}:{:>02}:{:>02}",
                    delta.num_hours(),
                    delta.num_minutes(),
                    num_seconds % 60
                )
            } else {
                format!("{}s", num_seconds)
            };
            Ok(tera::to_value(s).unwrap())
        },
    )
}
