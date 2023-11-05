#![allow(unused)]

use axum::{
    body::Bytes,
    extract::DefaultBodyLimit,
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, LAST_MODIFIED},
        HeaderValue, Method,
    },
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};
use calamine::{open_workbook_auto_from_rs, open_workbook_from_rs, Reader, Sheets, Xlsx};
use excel_merge::{ApiDoc, Data};
use serde::Deserialize;
use std::{
    io::{BufReader, Cursor},
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
pub mod error;
pub mod excel_merge;

#[derive(Clone)]
pub struct AppState {
    data: Arc<Mutex<Vec<Vec<serde_json::Map<String, serde_json::Value>>>>>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let state = AppState {
        data: Arc::new(Mutex::new(Vec::new())),
    };

    let router = Router::new()
        .route("/hello", post(hello))
        .route("/merge", post(excel_merge::merge_files))
        .route("/add", post(excel_merge::add_file))
        .with_state(state)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE, LAST_MODIFIED]),
        )
        .layer(DefaultBodyLimit::max(300 * 1000 * 1000));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("->> LISTENING on {addr}\n");
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();

    Ok(())
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    data: Vec<String>,
}

async fn hello(body: Bytes) -> impl IntoResponse {
    let bytes = body;
    let reader = Cursor::new(&bytes);
    let mut excel: Xlsx<_> = open_workbook_from_rs(reader).unwrap();
}
