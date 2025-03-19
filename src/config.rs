

#[derive(Debug, Clone)]
pub struct Config {
    pub app_data: String,
    pub bind: String,
    pub db: String
}

impl Config {

    pub fn from_env() -> Self {
        let db = std::env::var("DB").unwrap_or("./picam.db".to_string());
        let app_data = std::env::var("APP_DATA").unwrap_or("./app_data".to_string());
        let bind = std::env::var("BIND").unwrap_or("0.0.0.0:8080".to_string());

        Self {
            app_data,
            bind,
            db
        }
    }
}