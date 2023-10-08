use anyhow::Result;
use axum::{
    http::{
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
        HeaderValue, Method,
    },
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};

pub mod error;
pub mod whatever;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let router = Router::new()
        .route("/", get(hello))
        .route("/merge", post(whatever::merge_files))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]),
        );
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    info!("Listening on {}", addr);

    let cloned_router = router.clone();

    axum::Server::bind(&addr)
        .serve(cloned_router.into_make_service())
        .await
        .unwrap();

    Ok(router.into())
}

async fn hello() -> impl IntoResponse {
    Html(format!("Hello, World!"))
}
