use std::{path::PathBuf, str::FromStr, sync::Arc, time::SystemTime};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    sync::{broadcast::Receiver, RwLock},
};
use log::*;
use crate::{MessageType, ParsedBuffer};
pub const APP_DATA_PATH: &str = "./app_data";


pub struct FileSinkConfig {
    app_data: String,
    max_file_duration: Option<u64>,
    max_number_of_file: Option<u64>,
    max_system_usage: f64,
}

impl Default for FileSinkConfig {
    fn default() -> Self {
        const CREATE_NEW_FILE_THRESHOLD: u64 = 10 * 60;
        const MAX_FS_USAGE: f64 = 0.9;
        const MAX_NUMBER_OF_FILES: u64 = 24 * 60 * 60 / CREATE_NEW_FILE_THRESHOLD; // 1 - day
        pub const APP_DATA_PATH: &str = "./app_data";
        Self {
            app_data: APP_DATA_PATH.to_string(),
            max_file_duration: Some(CREATE_NEW_FILE_THRESHOLD),
            max_number_of_file: Some(MAX_NUMBER_OF_FILES),
            max_system_usage: MAX_FS_USAGE,
        }
    }
}

pub async fn file_saver(
    mut recv: Receiver<Arc<ParsedBuffer>>,
    moov: Arc<RwLock<Vec<Vec<u8>>>>,
    mut config_reciver: Receiver<Arc<FileSinkConfig>>,
) {
    let mut config = config_reciver.recv().await.unwrap_or_default();
    let mut file = generate_new_file(&config.app_data).await;
    // if let Err(_) = save_moov_header(&moov, &mut file).await {
    // TODO log and handle error
    // }

    let mut timestamp_when_file_is_created = 0;

    loop {
        tokio::select! {
            recv = recv.recv() => {
                match recv {
                    Ok(buffer) => {
                        if buffer.message_type == MessageType::FirstFrame ||
                    (
                        buffer.message_type == MessageType::KeyFrame &&
                        should_create_new_file(&buffer,&mut timestamp_when_file_is_created, &config)
                    ) {
                        while should_file_be_rotated(&config).await {
                            remove_oldest_file(&config.app_data).await;
                        }

                        file = generate_new_file(&config.app_data).await;
                        if let Err(_) = save_moov_header(&moov, &mut file).await {
                            // TODO log and handle error
                        }
                    }
                    if let Err(_ ) = file.write(&buffer.data).await {
                        // TODO log error and create new file
                    }
                },
                Err(e) => {
                    error!("Error when saving files, communication channel brokne {e:?}")
                }
            }
        },
        new_config = config_reciver.recv() => {
            if let Ok(new_config) = new_config {
                config = new_config;
            }

        }
        }
    }
}

async fn remove_oldest_file(app_data: &str) {
    let mut files = tokio::fs::read_dir(app_data).await.unwrap();

    let mut max_time = SystemTime::now();
    let mut path = PathBuf::new();
    while let Ok(Some(file)) = files.next_entry().await {
        if let Ok(metadata) = file.metadata().await {
            if metadata.is_file() && file.path().extension().map(|e| e == "mp4").unwrap_or(false) {
                let time_created = metadata.created().unwrap();

                if time_created < max_time {
                    max_time = time_created;
                    path = file.path();
                }
            }
        }
    }

    if let Err(_) = tokio::fs::remove_file(path).await {
        // TODO log error and handle it
    }
}

async fn should_file_be_rotated(config: &FileSinkConfig) -> bool {
    percentage_of_file_system_usage(&config.app_data) > config.max_system_usage
        || (config.max_number_of_file.is_some()
            && &number_of_mp4_files(&config.app_data).await
                > config.max_number_of_file.as_ref().unwrap_or(&0))
}

pub async fn number_of_mp4_files(app_data: &str) -> u64 {
    let mut files = tokio::fs::read_dir(app_data).await.unwrap();
    let mut number_of_mp4_files = 0;
    while let Ok(Some(file)) = files.next_entry().await {
        if file
            .file_name()
            .to_str()
            .map(|s| s.ends_with(".mp4"))
            .unwrap_or(false)
        {
            number_of_mp4_files += 1;
        }
    }
    number_of_mp4_files
}

pub fn percentage_of_file_system_usage(app_data: &str) -> f64 {
    let total_space = fs2::total_space(app_data).unwrap_or(1);
    let free_space = fs2::free_space(app_data).unwrap_or(1);
    let ret = 1.0 - (free_space as f64 / total_space as f64);
    return ret;
}

fn should_create_new_file(
    buffer: &Arc<ParsedBuffer>,
    base_timestamp: &mut u64,
    config: &FileSinkConfig,
) -> bool {
    if let Some(ref timestamp) = buffer.timestamp {
        let ts = timestamp.seconds();
        if let Some(ref max_file_duration) = config.max_file_duration {
            if ts.abs_diff(*base_timestamp) > *max_file_duration {
                *base_timestamp = ts;
                return true;
            }
        }
    }

    false
}

async fn save_moov_header(
    moov: &Arc<RwLock<Vec<Vec<u8>>>>,
    file: &mut File,
) -> Result<(), std::io::Error> {
    for header in moov.read().await.iter() {
        file.write(&header).await?;
    }
    Ok(())
}

fn generate_file_name() -> String {
    format!("{}.mp4", chrono::Local::now())
}

async fn generate_new_file(app_data: &str) -> File {
    let file_name = generate_file_name();
    let file_path = std::path::PathBuf::from_str(&format!("{app_data}/{file_name}")).unwrap();
    File::create_new(file_path).await.unwrap() // TODO: Handle errors
}
