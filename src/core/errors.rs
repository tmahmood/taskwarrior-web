use std::collections::HashMap;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

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

impl ToString for AppError {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormValidation {
    pub fields: HashMap<String, Vec<FieldError>>,
    pub msg: Option<String>,
    success: bool,
}

impl Default for FormValidation {
    fn default() -> Self {
        Self {
            fields: Default::default(),
            msg: None,
            success: true,
        }
    }
}

impl std::fmt::Display for FormValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let field_list: Vec<String> = self.fields.values().map(|f| {
            let msg_list: Vec<String> = f.iter().map(|x| format!("{}={}", x.field.to_string(), x.message.to_string())).collect();
            msg_list.join(", ")
        }).collect();
        write!(f, "FormValidation error was {:?}, affected fields: {}", self.success, field_list.join("; "))
    }
}

impl std::error::Error for FormValidation {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

impl From<anyhow::Error> for FormValidation {
    fn from(value: anyhow::Error) -> Self {
        Self::default().set_error(Some(&value.to_string())).to_owned()
    }
}

impl From<taskchampion::Error> for FormValidation {
    fn from(value: taskchampion::Error) -> Self {
        Self::default().set_error(Some(&value.to_string())).to_owned()
    }
}

impl FormValidation {
    pub fn push(&mut self, error: FieldError) -> () {
        self.success = false;
        if let Some(val) = self.fields.get_mut(&error.field) {
            val.push(error);
        } else {
            self.fields.insert(error.field.to_string(), vec![error]);
        }
    }

    /// Check if any validation errors occured or if no errors were recognized.
    /// If everything went fine, `is_success` returns `true`.
    pub fn is_success(&self) -> bool {
        self.success
    }

    pub fn set_error(&mut self, msg: Option<&str>) -> &Self {
        if let Some(err_msg) = msg {
            self.success = false;
            self.msg = Some(err_msg.to_string());
        } else {
            self.success = !self.fields.is_empty();
            self.msg = None;
        }

        self
    }

    /// Checks whether errors occured for given `field`.
    /// If at least one error to the given `field`, a `true` 
    /// is returned.
    pub fn has_error(&self, field: &str) -> bool {
        self.fields.contains_key(field)
    }

}
