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
use excel_merge::error::Result;
use excel_merge::routes;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(askama::Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    // name: String,
}

#[tokio::main]
//TODO: Factor all the Vecs to slices, and Strings to AsRef<&str>
async fn main() {
    let trace_sub = tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(EnvFilter::new("excel_merge=debug"))
        .finish();

    tracing::subscriber::set_global_default(trace_sub).unwrap();

    // FIXME: fix swagger ui
    let router = Router::new()
        .route("/merge", post(routes::merge::merge_files))
        .route("/", get(index))
        // .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        .layer(
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE, LAST_MODIFIED]),
        )
        .layer(DefaultBodyLimit::max(800 * 1000 * 1000));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    info!("->> LISTENING on {addr}\n");

    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}

async fn index() -> Result<impl IntoResponse> {
    println!("Hello, world!");
    let template = IndexTemplate {};
    Ok(template)
}
