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
    let buffer = FilesMap::merge_from_multipart(multipart)
        .await?
        .write_to_buffer()?;

    Ok(buffer)
}
