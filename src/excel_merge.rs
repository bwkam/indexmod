use crate::error::Result;
use axum::{extract::Multipart, response::IntoResponse};
// use axum_macros::debug_handler;
use crate::FilesMap;
use tracing::info;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(paths(merge_files))]
pub struct ApiDoc;

#[utoipa::path(
    get,
    path = "/merge",
    responses(
        (status = 200, description = "Merge Excel files")
    )
)]
// TODO: Add an #[instrument] for span tracing
pub async fn merge_files(multipart: Multipart) -> Result<impl IntoResponse> {
    info!("Merge requested. Processing files...");

    // create the files map object that will handle the merging
    let mut files_map = FilesMap::new(multipart).await?;

    // merge the files and save to a buffer
    let merged_buf = files_map.save_to_buf()?;

    Ok(merged_buf)
}
