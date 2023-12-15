use axum::response::IntoResponse;
use axum::{
    extract::DefaultBodyLimit,
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, LAST_MODIFIED},
        HeaderValue, Method,
    },
    routing::get,
    routing::post,
    Router,
};
use error::AppError;
// use axum_macros::debug_handler;
use excel_merge::ApiDoc;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
pub mod error;
pub mod excel_merge;

#[derive(askama::Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    // name: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let router = Router::new()
        .route("/merge", post(excel_merge::merge_files))
        .route("/", get(index))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE, LAST_MODIFIED]),
        )
        .layer(DefaultBodyLimit::max(800 * 1000 * 1000));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("->> LISTENING on {addr}\n");
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();

    Ok(())
}

async fn index() -> Result<impl IntoResponse, AppError> {
    println!("Hello, world!");
    let template = IndexTemplate {
        // name: "world".to_string(),
    };
    Ok(template)
}
