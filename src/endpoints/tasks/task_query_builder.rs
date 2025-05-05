use std::cmp::PartialEq;
use std::fmt::{Display, Formatter};
use std::process::Command;
use serde::{Deserialize, Serialize};
use tracing::log::trace;
use crate::TWGlobalState;

pub enum TQUpdateTypes {
    Priority(String),
    Status(String),
    Report(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TaskReport {
    Next,
    New,
    Ready,
    All,
    NotSet,
}

impl Display for TaskReport {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            TaskReport::Next => "next",
            TaskReport::New => "new",
            TaskReport::Ready => "ready",
            TaskReport::All => "all",
            TaskReport::NotSet => ""
        })
    }
}

impl From<String> for TaskReport {
    fn from(value: String) -> Self {
        match value.as_str() {
            "ready" => TaskReport::Ready,
            "new" => TaskReport::New,
            "next" => TaskReport::Next,
            "all" => TaskReport::All,
            _ => TaskReport::NotSet
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum TaskPriority {
    High,
    Medium,
    Low,
    NotSet,
}

impl From<String> for TaskPriority {
    fn from(value: String) -> Self {
        match value.as_str() {
            "H" => TaskPriority::High,
            "M" => TaskPriority::Medium,
            "L" => TaskPriority::Low,
            "priority:H" => TaskPriority::High,
            "priority:M" => TaskPriority::Medium,
            "priority:L" => TaskPriority::Low,
            _ => TaskPriority::NotSet
        }
    }
}

impl Display for TaskPriority {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            TaskPriority::High => "priority:H",
            TaskPriority::Medium => "priority:M",
            TaskPriority::Low => "priority:L",
            TaskPriority::NotSet => ""
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum TaskStatus {
    Pending,
    Completed,
    Waiting,
    NotSet,
}

impl From<String> for TaskStatus {
    fn from(value: String) -> Self {
        match value.as_str() {
            "pending" => TaskStatus::Pending,
            "completed" => TaskStatus::Completed,
            "waiting" => TaskStatus::Waiting,
            "status:pending" => TaskStatus::Pending,
            "status:completed" => TaskStatus::Completed,
            "status:waiting" => TaskStatus::Waiting,
            _ => TaskStatus::NotSet
        }
    }
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            TaskStatus::Pending => "status:pending",
            TaskStatus::Completed => "status:completed",
            TaskStatus::Waiting => "status:waiting",
            TaskStatus::NotSet => ""
        })
    }
}

// this will get the params and build task command
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskQuery {
    status: TaskStatus,
    priority: TaskPriority,
    report: TaskReport,
    tags: Vec<String>,
    project: Option<String>,
    filter: Option<String>,
    new_entry: Option<String>,
    custom_query: Option<String>,
}

impl Default for TaskQuery {
    fn default() -> Self {
        TaskQuery {
            status: TaskStatus::NotSet,
            priority: TaskPriority::NotSet,
            report: TaskReport::Next,
            tags: vec![],
            project: None,
            filter: None,
            new_entry: None,
            custom_query: None,
        }
    }
}

impl TaskQuery {
    pub fn new(params: TWGlobalState) -> Self {
        let mut tq = Self::default();
        tq.update(params);
        tq
    }

    pub fn all() -> Self {
        TaskQuery {
            status: TaskStatus::NotSet,
            priority: TaskPriority::NotSet,
            report: TaskReport::All,
            tags: vec![],
            project: None,
            filter: None,
            new_entry: None,
            custom_query: None,
        }
    }

    pub fn empty() -> Self {
        TaskQuery {
            status: TaskStatus::NotSet,
            priority: TaskPriority::NotSet,
            report: TaskReport::NotSet,
            tags: vec![],
            project: None,
            filter: None,
            new_entry: None,
            custom_query: None,
        }
    }

    pub fn update(&mut self, params: TWGlobalState) {
        if params.report.as_ref().is_none() && params.status.as_ref().is_none() {
            if params.custom_query.as_ref().is_some_and(|f| !f.is_empty()) {
                self.report = TaskReport::NotSet;
                self.custom_query = params.custom_query;
                self.filter = params.filter;
            }
        } else {
            self.custom_query = None;
            self.filter = None;
        }

        if let Some(rep) = params.report {
            self.report = rep.into();
            self.status = TaskStatus::NotSet;
        }

        if let Some(status) = params.status {
            let s: TaskStatus = status.into();
            if s == self.status {
                self.status = TaskStatus::NotSet;
                self.report = TaskReport::Next;
            } else {
                self.status = s;
                self.report = TaskReport::NotSet;
            }
        }

        if let Some(t) = params.query {
            if t.starts_with("project:") {
                if self.project == Some(t.clone()) {
                    self.project = None;
                } else {
                    self.project = Some(t);
                }
            } else if t.starts_with("+") {
                if self.tags.contains(&t) {
                    self.tags.retain_mut(|iv| iv != &t);
                } else {
                    self.tags.push(t);
                }
            } else if t.starts_with("priority:") {
                let tp: TaskPriority = t.clone().into();
                if self.priority == tp {
                    self.priority = TaskPriority::NotSet;
                } else {
                    self.priority = tp;
                }
            }
        }
        self.new_entry = params.task_entry;
        trace!("{:?}", self);
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = Some(filter.to_string())
    }

    pub fn get_query(&self, with_export: bool) -> Vec<String> {
        let mut output = vec![];
        let mut export_suffix = vec![];
        let mut export_prefix = vec![];
        if let Some(f) = &self.filter.clone() {
            let task_filter = shell_words::split(f);
            if let Ok(task_filter) = task_filter {
                export_prefix.extend(task_filter);
            }
        }
        match &self.report {
            TaskReport::NotSet => {}
            v => {
                export_suffix.push(v.to_string())
            }
        }
        match &self.priority {
            TaskPriority::NotSet => {}
            v => {
                export_prefix.push(v.to_string())
            }
        }
        if let Some(p) = self.project.clone() {
            export_prefix.push(p)
        }
        if self.tags.len() > 0 {
            export_prefix.extend(self.tags.clone())
        }
        match &self.status {
            TaskStatus::NotSet => {}
            v => {
                export_prefix.push(v.to_string())
            }
        }
        if let Some(e) = self.new_entry.clone() {
            export_prefix.push(e);
        }
        output.extend(export_prefix);
        if with_export {
            output.extend(vec!["export".to_string()]);
        }
        output.extend(export_suffix);
        output
    }

    pub fn as_filter_text(&self) -> Vec<String> {
        self.get_query(false)
    }

    pub fn build(&self) -> Command {
        let mut task = Command::new("task");
        let output = self.get_query(true);
        task.args(&output);
        task
    }
    pub fn status(&self) -> &TaskStatus {
        &self.status
    }
    pub fn priority(&self) -> &TaskPriority {
        &self.priority
    }
    pub fn report(&self) -> &TaskReport {
        &self.report
    }
    pub fn tags(&self) -> &Vec<String> {
        &self.tags
    }
    pub fn project(&self) -> &Option<String> {
        &self.project
    }
    pub fn filter(&self) -> &Option<String> {
        &self.filter
    }
    pub fn new_entry(&self) -> &Option<String> {
        &self.new_entry
    }
    pub fn custom_query(&self) -> &Option<String> {
        &self.custom_query
    }
}

#[cfg(test)]
mod tests;


