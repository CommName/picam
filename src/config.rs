

#[derive(Debug, Clone)]
pub struct Config {
    pub source: String,
}

impl Config {
    pub fn from_env() -> Self {
        let source = std::env::var("SOURCE")
            .unwrap_or_else(|_| String::from("/dev/video0"));
        Self {
            source
        }
    }
}