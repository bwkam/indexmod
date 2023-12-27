use crate::error::Result;
use crate::FilesMap;
use axum::{extract::Multipart, response::IntoResponse};
use tracing::info;

pub mod template_download;

// TODO: Add an #[instrument] for span tracing
#[utoipa::path(
    get,
    path = "/search",
    responses(
        (status = 200, description = "Query excel files")
    )
)]
pub async fn search_files(multipart: Multipart) -> Result<impl IntoResponse> {
    info!("Search requested. Processing files...");

    // create the files map object that will handle the merging
    let buffer = FilesMap::search_from_multipart(multipart)
        .await?
        .write_to_buffer()?;

    Ok(buffer)
}
