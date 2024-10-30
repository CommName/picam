use poem_openapi::{payload::Json, OpenApi};


pub struct Api;

#[OpenApi]
impl Api {
    /// Hello world
    #[oai(path = "/recordings", method = "get")]
    async fn index(&self) -> Json<Vec<String>> {
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
}
