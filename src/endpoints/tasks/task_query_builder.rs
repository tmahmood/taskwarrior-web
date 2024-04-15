use std::fmt::{Display, Formatter};
use std::process::Command;
use serde::{Deserialize, Serialize};
use crate::Params;

pub enum TQUpdateTypes {
    Priority(String),
    Status(String),
    Report(String),
}

#[derive(Debug, Serialize, Deserialize)]
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


#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Serialize, Deserialize, Debug)]
pub struct TaskQuery {
    status: TaskStatus,
    priority: TaskPriority,
    report: TaskReport,
    tags: Vec<String>,
    project: Option<String>,
}


impl TaskQuery {

    pub fn new(params: Params) -> Self {
        let mut tq = TaskQuery {
            status: TaskStatus::NotSet,
            priority: TaskPriority::NotSet,
            report: TaskReport::Next,
            tags: vec![],
            project: None,
        };
        tq.update(params);
        tq
    }

    pub fn update(&mut self, params: Params) {
        if let Some(rep) = params.report {
            self.report = rep.into();
        }
        if let Some(status) = params.status {
            self.status = status.into();
            self.report = TaskReport::NotSet;
        }
        if let Some(p) = params.priority {
            self.priority = p.clone().into();
        }
        if let Some(t) = params.q {
            if t.starts_with("project:") {
                self.project = Some(t);
            } else if t.starts_with("+") {
                self.tags.push(t);
            }
        }
        println!("{:?}", self);
    }

    pub fn build(&self) -> Command {
        let mut task = Command::new("task");
        let mut output = vec![];
        let mut export_suffix = vec![];
        let mut export_prefix = vec![];
        match &self.report {
            TaskReport::NotSet => {}
            (v) => {
                export_suffix.push(v.to_string())
            }
        }
        match &self.priority {
            TaskPriority::NotSet => {}
            (v) => {
                export_prefix.push(v.to_string())
            }
        }
        if let Some(p) = self.project.clone() {
            export_prefix.push(p)
        }
        if self.tags.len() > 0 {
            export_prefix.push(self.tags.join(" "))
        }
        match &self.status {
            TaskStatus::NotSet => {}
            (v) => {
                export_prefix.push(v.to_string())
            }
        }
        output.extend(export_prefix);
        output.extend(vec!["export".to_string()]);
        output.extend(export_suffix);
        task.args(&output);
        task
    }

}

#[cfg(test)]
mod tests;


