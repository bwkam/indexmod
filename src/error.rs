use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// for ?
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO: Use the right status codes for the right errors
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Only one sheet per file is supported")]
    SheetLimitExceeded,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status_code, body) = (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!(
                {
                    "error": self.to_string(),
                }
            )),
        );

        (status_code, body).into_response()
    }
}
