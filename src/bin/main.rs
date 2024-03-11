use anyhow::Context;
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

use excel_merge::error::{Result, self};
use excel_merge::routes;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::info;

static VERSION: &str = "3.6";

#[derive(askama::Template)]
#[template(path = "merge.html")]
struct MergeTemplate {}

#[derive(askama::Template)]
#[template(path = "search.html")]
struct SearchTemplate {}
#[tokio::main]
async fn main() -> error::Result<()> {
    // log control
    std::env::set_var("RUST_LOG", "trace");

    // setup tracing
    tracing_subscriber::fmt::init();

    info!("using version: {:?}", VERSION);

    // serve static files
    let serve_dir = ServeDir::new("assets").not_found_service(ServeFile::new("assets/index.html"));

    // FIXME: fix swagger ui
    let router = Router::new()
        // TODO: make a seperate router for api
        .route("/api/merge", post(routes::merge::merge_files))
        .route("/api/search", post(routes::search::search_files))
        .route(
            "/api/search/download_template",
            post(routes::search::template_download::download),
        )
        .route("/merge", get(merge))
        .route("/search", get(search))
        .nest_service("/_assets", serve_dir.clone())
        .fallback_service(serve_dir)
        // .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE, LAST_MODIFIED]),
        )
        .layer(DefaultBodyLimit::max(800 * 1000 * 1000));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    info!("->> LISTENING on {:?}", addr);

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .context("error launching server")?;

    Ok(())
}

async fn merge() -> Result<impl IntoResponse> {
    let template = MergeTemplate {};
    Ok(template)
}

async fn search() -> Result<impl IntoResponse> {
    let template = SearchTemplate {};
    Ok(template)
}
