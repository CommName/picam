use serde::{Deserialize, Serialize};



#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileSinkConfig {
    max_file_duration: Option<u64>,
    max_number_of_file: Option<u64>,
    max_system_usage: f64,
}

impl Default for FileSinkConfig {
    fn default() -> Self {
        Self {
            max_file_duration: Default::default(),
            max_number_of_file: Default::default(),
            max_system_usage: 0.9
        }    
    }
}