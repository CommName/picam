use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use sqlx::{prelude::FromRow, Decode, Encode};

#[derive(Object, Debug, Clone, Encode, Decode, Default, FromRow, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub source: Option<String>,
    pub use_cam_builtin_encoder: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>
}
