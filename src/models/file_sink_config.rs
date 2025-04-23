use poem_openapi::Object;
use serde::{Deserialize, Serialize};



#[derive(Object, Serialize, Deserialize, Debug, Clone)]
pub struct FileSinkConfig {
    pub max_file_duration: Option<u64>,
    pub max_number_of_file: Option<u64>,
    pub max_system_usage: f64,
}

impl Default for FileSinkConfig {
    fn default() -> Self {
        const CREATE_NEW_FILE_THRESHOLD: u64 = 10 * 60;

        Self {
            max_file_duration: Some(CREATE_NEW_FILE_THRESHOLD),
            max_number_of_file: None,
            max_system_usage: 0.9
        }    
    }
}