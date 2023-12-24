use axum::extract::Path;
use axum::http::{header, HeaderMap, StatusCode};
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
#[template(path = "merge.html")]
struct MergeTemplate {
    // name: String,
}

#[derive(askama::Template)]
#[template(path = "new_file.html")]
struct NewFileTemplate {
    name: String,
    id: String,
    file: String,
    date: String,
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
        .route("/api/merge", post(routes::merge::merge_files))
        .route("/api/new_file", post(new_file))
        .route("/merge", get(merge))
        .route("/_assets/*path", get(assets))
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

async fn merge() -> Result<impl IntoResponse> {
    let template = MergeTemplate {};
    Ok(template)
}

async fn assets(Path(path): Path<String>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    let content = tokio::fs::read_to_string(format!("./assets/{}", path)).await;

    match content {
        Ok(content) => {
            if path.ends_with(".css") {
                headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
            } else if path.ends_with(".js") {
                headers.insert(header::CONTENT_TYPE, "text/javascript".parse().unwrap());
            } else if path.ends_with(".svg") {
                headers.insert(header::CONTENT_TYPE, "image/svg+xml".parse().unwrap());
            }

            (StatusCode::OK, headers, content)
        }
        Err(_) => (StatusCode::NOT_FOUND, headers, "".to_string()),
    }
}

async fn new_file() -> Result<impl IntoResponse> {
    let template = NewFileTemplate {
        name: "yo".to_string(),
        id: "yo".to_string(),
        file: "yo".to_string(),
        date: "yo".to_string(),
    };
    Ok(template)
}
