use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ErrorBodyDoc {
    pub code: String,
    pub message: String,
}
