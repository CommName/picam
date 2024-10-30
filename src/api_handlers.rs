use std::{path::PathBuf, str::FromStr};

use poem_openapi::{param::Path, payload::{Binary, Json, Response}, OpenApi};


pub struct Api;

#[OpenApi]
impl Api {
    /// Hello world
    #[oai(path = "/recordings", method = "get")]
    async fn list_recordings(&self) -> Json<Vec<String>> {
        let mut recordings = Vec::new();

        let mut dir = tokio::fs::read_dir(crate::file_sink::APP_DATA_PATH).await
            .unwrap();

        while let Ok(Some(f)) = dir.next_entry().await {
            let path = f.path();
            if path.extension().map(|e| e == "mp4").unwrap_or(false) {
                recordings.push(path.file_name().unwrap().to_str().unwrap().to_string());
            }
        }
        
        Json(recordings)
    }


    #[oai(path = "/recordings/:recording", method = "get")]
    async fn download_recordings(&self, Path(recording): Path<String>) -> Response<Binary<Vec<u8>>> {

        let mut path = PathBuf::from_str(&(crate::file_sink::APP_DATA_PATH.to_string() + "/" + &recording))
            .unwrap(); // TODO handle this error

        if path.parent().map(|p| p.as_os_str() != crate::file_sink::APP_DATA_PATH).unwrap_or(true) {
            println!("Tempered path from user side");
        }
        // TODO check if file exists

        let content = tokio::fs::read(path).await
            .unwrap(); // TODO handle this error

        Response::new(Binary(content)).header(poem::http::header::CONTENT_TYPE, "video/mp4")
    }
}
