use axum::{
    extract::DefaultBodyLimit,
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use whatever::ApiDoc;
pub mod error;
pub mod whatever;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(hello))
        .route("/merge", post(whatever::merge_files))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]),
        )
        .layer(DefaultBodyLimit::max(40 * 1000 * 1000));

    Ok(router.into())
}

async fn hello() -> impl IntoResponse {
    Html(format!("Hello, World!"))
}
