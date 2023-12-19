use crate::error::Result;
use crate::FilesMap;
use axum::{extract::Multipart, response::IntoResponse};
use tracing::info;

// TODO: Add an #[instrument] for span tracing
#[utoipa::path(
    get,
    path = "/merge",
    responses(
        (status = 200, description = "Merge Excel files")
    )
)]
pub async fn merge_files(multipart: Multipart) -> Result<impl IntoResponse> {
    info!("Merge requested. Processing files...");

    // create the files map object that will handle the merging
    let mut files_map = FilesMap::from_multipart(multipart).await?;

    // merge the files and save to a buffer
    let merged_buf = files_map.save_to_buf()?;

    Ok(merged_buf)
}
