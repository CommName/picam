use poem_openapi::Object;

#[derive(Object, Debug, Clone)]
pub struct PipelineConfig {
    pub source: Option<String>,
    pub use_cam_builtin_encoder: Option<bool>,
    pub width: Option<u32>,
    pub height: Option<u32>
}
