#![allow(unused)]

use std::{collections::HashMap, fs::File, io::BufReader, ops::Add};

use anyhow::{Error, Result};
use axum::{
    body::Bytes,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    Json,
};
use rust_xlsxwriter::Workbook;
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct Data {
    data: Vec<Vec<serde_json::Map<String, serde_json::Value>>>, // fix this
}

// Make our own error that wraps `anyhow::Error`.
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

pub async fn merge_files(Json(data): Json<Data>) -> Result<impl IntoResponse, AppError> {
    info!("Received data: {:?}", data);

    let vec: Vec<serde_json::Map<String, serde_json::Value>> =
        data.data.into_iter().flat_map(|file| file).collect();

    Ok(Json(vec))
}
