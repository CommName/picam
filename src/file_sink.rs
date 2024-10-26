use std::sync::Arc;

use tokio::{fs::File, io::AsyncWriteExt, sync::broadcast::{Receiver, Sender}};



pub struct  FileSinkConfig {

    duration_of_file_in_seconds: u32,
    max_file_size: u32,
    rotation_period_in_seconds: u32,
    max_disk_usage: u32,
    max_percentage_of_disk_usage: u32,

}


struct State {
    current_timestamps: u32,

}

pub async  fn file_saver(mut recv: Receiver<Vec<u8>>, moov: Arc<Vec<Vec<u8>>>) {

    let mut file = generate_new_file().await;
    save_moov_header(&moov, &mut file).await;

    while let Ok(bytes) = recv.recv().await {
        file.write(&bytes).await;
    }

}

pub async fn save_moov_header(moov: &Arc<Vec<Vec<u8>>>, file: &mut File) {
    for header in moov.iter() {
        file.write(&header).await;
    }
}

pub async fn generate_new_file() -> File {
    File::create_new("./file.mp4")
        .await
        .unwrap() // TODO: Handle errors
}

