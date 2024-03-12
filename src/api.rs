#[derive(utoipa::OpenApi)]
#[openapi(paths(crate::routes::merge::merge_files, crate::routes::search::search_files))]
pub struct ApiDoc;

