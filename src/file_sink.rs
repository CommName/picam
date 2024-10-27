use std::{str::FromStr, sync::Arc};

use tokio::{fs::File, io::AsyncWriteExt, sync::broadcast::Receiver};

use crate::ParsedBuffer;


pub async  fn file_saver(mut recv: Receiver<Arc<ParsedBuffer>>, moov: Arc<Vec<Vec<u8>>>) {

    let mut file = generate_new_file().await;
    if let Err(_) = save_moov_header(&moov, &mut file).await {
        // TODO log and handle error
    }

    while let Ok(buffer) = recv.recv().await {
        if let Err(_ ) = file.write(&buffer.data).await {
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

pub fn generate_file_name() -> String {
    format!("{}.mp4", chrono::Local::now())
}

pub async fn generate_new_file() -> File {
    let file_name = generate_file_name();
    let file_path = std::path::PathBuf::from_str(&format!("./{file_name}")).unwrap();
    File::create_new(file_path)
        .await
        .unwrap() // TODO: Handle errors
}

