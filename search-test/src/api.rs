#[derive(utoipa::OpenApi)]
#[openapi(paths(crate::routes::merge::merge_files))]
pub struct ApiDoc;
