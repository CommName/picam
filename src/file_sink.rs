use std::{path::PathBuf, str::FromStr, sync::Arc, time::SystemTime};

use tokio::{fs::File, io::AsyncWriteExt, sync::broadcast::Receiver};

use crate::ParsedBuffer;

const CREATE_NEW_FILE_THRESHOLD: u64 = 5 * 60;
const MAX_NUMBER_OF_FILES: u64 = 24 * 60 * 60 / CREATE_NEW_FILE_THRESHOLD; // 1 - day
pub const APP_DATA_PATH: &str = "./app_data";


pub struct FileSinkConfig {
    app_data: String,
    max_file_duration: Option<u64>,
    max_number_of_file: Option<u64>
}

pub async  fn file_saver(mut recv: Receiver<Arc<ParsedBuffer>>, moov: Arc<Vec<Vec<u8>>>) {

    let mut file = generate_new_file().await;
    if let Err(_) = save_moov_header(&moov, &mut file).await {
        // TODO log and handle error
    }

    let mut timestamp_when_file_is_created = 0;

    while let Ok(buffer) = recv.recv().await {
        if buffer.key_frame && should_create_new_file(&buffer,&mut timestamp_when_file_is_created) {
            while should_file_be_rotated().await {
                remove_oldest_file().await;
            }

            file = generate_new_file().await;
            if let Err(_) = save_moov_header(&moov, &mut file).await {
                // TODO log and handle error
            }
        }
        if let Err(_ ) = file.write(&buffer.data).await {
            // TODO log error and create new file
        }
    }

}

async fn remove_oldest_file() {
    let mut files = tokio::fs::read_dir(APP_DATA_PATH).await.unwrap();

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

async fn should_file_be_rotated() -> bool {
    let mut files = tokio::fs::read_dir(APP_DATA_PATH).await.unwrap();

    let mut number_of_mp4_files = 0;
    while let Ok(Some(file)) = files.next_entry().await {
        if file.file_name()
            .to_str()
            .map(|s| s.ends_with(".mp4"))
            .unwrap_or(false) {

            number_of_mp4_files += 1;
        }
    }
    number_of_mp4_files > MAX_NUMBER_OF_FILES
}

fn should_create_new_file(buffer: &Arc<ParsedBuffer>, base_timestamp: &mut u64) -> bool {
    if let Some(ref timestamp) = buffer.timestamp {
        let ts = timestamp.seconds();
        if ts.abs_diff(*base_timestamp) > CREATE_NEW_FILE_THRESHOLD {
            *base_timestamp = ts;
            return  true;
        }
    }

    false
}

async fn save_moov_header(moov: &Arc<Vec<Vec<u8>>>, file: &mut File) -> Result<(), std::io::Error> {
    for header in moov.iter() {
        file.write(&header).await?;
    }
    Ok(())
}

fn generate_file_name() -> String {
    format!("{}.mp4", chrono::Local::now())
}

async fn generate_new_file() -> File {
    let file_name = generate_file_name();
    let file_path = std::path::PathBuf::from_str(&format!("{APP_DATA_PATH}/{file_name}")).unwrap();
    File::create_new(file_path)
        .await
        .unwrap() // TODO: Handle errors
}

