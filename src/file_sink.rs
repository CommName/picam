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

pub async  fn file_saver(mut recv: Receiver<Vec<u8>>, moov: &Vec<u8>) {

    let mut file = generate_new_file().await;
    file.write(&moov).await;

    while let Ok(bytes) = recv.recv().await {
        
    }

}

pub async fn generate_new_file() -> File {
    File::create_new("./file.mp4")
        .await
        .unwrap() // TODO: Handle errors
}

