use poem_openapi::Object;
use serde::{Deserialize, Serialize};


#[derive(Object, Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub password: String
}


#[derive(Object, Debug, Clone)]
pub struct PipelineConfig {
    pub source: Option<String>,
    pub use_cam_builtin_encoder: Option<bool>,
    pub width: Option<i32>,
    pub height: Option<i32>
}
