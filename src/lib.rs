use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{de, Deserialize, Deserializer};
use std::str::FromStr;
use std::fmt;
use serde::de::Error;
use tracing::trace;

pub fn add(left: usize, right: usize) -> usize {
    left + right
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct Params {
    query: Option<String>,
}

impl Params {
    pub fn query(&self) -> Vec<&str> {
        trace!("{:?}", self.query);
        if let Some(tlist) = self.query.as_ref() {
            if tlist == "[ALL]" {
                vec![]
            } else {
                tlist.trim()
                    .split(" ")
                    .filter(|v| *v != " ")
                    .map(|v| v.trim())
                    .collect()
            }
        } else {
            vec![]
        }
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
