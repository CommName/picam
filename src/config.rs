

#[derive(Debug, Clone)]
pub struct Config {
    pub source: String,
    pub use_cam_builtin_encoder: bool,
    pub width: i32,
    pub height: i32
}

impl Config {
    pub fn from_env() -> Self {
        let source = std::env::var("SOURCE")
            .unwrap_or_else(|_| String::from("/dev/video0"));

        let use_cam_builtin_encoder = std::env::var("BUILT_IN_ENCODER")
            .map(|p| p.parse::<bool>().ok())
            .ok()
            .flatten()
            .unwrap_or(true);

        let width = std::env::var("WIDTH")
            .map(|p| p.parse::<i32>().ok())
            .ok()
            .flatten()
            .unwrap_or(1280);

        let height = std::env::var("HEIGHT")
            .map(|p| p.parse::<i32>().ok())
            .ok()
            .flatten()
            .unwrap_or(720);
        Self {
            source,
            use_cam_builtin_encoder,
            width,
            height
        }
    }
}