use crate::error::Result;
use crate::FilesMap;
use axum::{extract::Multipart, response::IntoResponse};
use tracing::info;

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
    let mut files_map = FilesMap::from_multipart(multipart).await?;

    // merge the files and save to a buffer
    let merged_buf = files_map.merge()?.write_to_buffer()?;

    Ok(merged_buf)
}
