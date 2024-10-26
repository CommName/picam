use std::sync::Arc;

use tokio::{fs::File, io::AsyncWriteExt, sync::broadcast::Receiver};


pub async  fn file_saver(mut recv: Receiver<Vec<u8>>, moov: Arc<Vec<Vec<u8>>>) {

    let mut file = generate_new_file().await;
    if let Err(_) = save_moov_header(&moov, &mut file).await {
        // TODO log and handle error
    }

    while let Ok(bytes) = recv.recv().await {
        if let Err(_ ) = file.write(&bytes).await {
            // TODO log error and create new file
        }
    }

}

pub async fn save_moov_header(moov: &Arc<Vec<Vec<u8>>>, file: &mut File) -> Result<(), std::io::Error> {
    for header in moov.iter() {
        file.write(&header).await?;
    }
    Ok(())
}

pub async fn generate_new_file() -> File {
    File::create_new("./file.mp4")
        .await
        .unwrap() // TODO: Handle errors
}

