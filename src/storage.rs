use std::{any::Any, collections::HashMap};

use crate::{models::*, sys::Device};

pub mod sqlite;
pub mod memory;

pub struct Storage {
    pub users: Box<dyn UserStorage + Send + Sync>,
    pub camera_config: Box<dyn PipelineConfigStorage + Send + Sync>,
    pub devices: Box<dyn DeviceStorage + Send + Sync>
}

impl Storage {

    pub async fn new_sqlite(sqlite_path: &str) -> Self {   
        let (
            users,
            camera_config
        ) = sqlite::sqlite_storage(&sqlite_path).await;

        let devices = Box::new(memory::MemoryDeviceStorage::default());

        Self {
            users,
            camera_config,
            devices
        }        
    }

}

#[async_trait::async_trait]
pub trait UserStorage {
    async fn get_users(&self ) -> Vec<User>;
    async fn get_user(&self, username: &str) -> Option<User>;
    async fn number_of_users(&self) -> usize;
    async fn update_user(&self, user: &User);
    async fn create_user(&self, user: &User);
    async fn delete_user(&self, username: &str);
}

// TODO turn this into macro
#[async_trait::async_trait]
pub trait PipelineConfigStorage {
    async fn pipeline_config(&self) -> PipelineConfig {
        let (
            width,
            height, 
            use_cam_builtin_encoder, 
            source
        ) = tokio::join!(
            self.width(), 
            self.height(), 
            self.use_cam_builtin_encoder(), 
            self.source()
        );
        PipelineConfig {
            width,
            height,
            source,
            use_cam_builtin_encoder
        }
    }
    async fn set_pipeline_config(&self, config: &PipelineConfig) {
        tokio::join!(
            self.set_height(config.height),
            self.set_width(config.width),
            self.set_use_cam_builtin_encoder(config.use_cam_builtin_encoder),
            self.set_source(config.source.as_ref().map(|x| x.as_str()))
        );
    }

    async fn width(&self) -> Option<u32>;
    async fn height(&self) -> Option<u32>;
    async fn use_cam_builtin_encoder(&self) -> Option<bool>;
    async fn source(&self) -> Option<String>;

    async fn set_width(&self, width: Option<u32>);
    async fn set_height(&self, height: Option<u32>);
    async fn set_use_cam_builtin_encoder(&self, built_in_encoder: Option<bool>);
    async fn set_source(&self, source: Option<&str>);
    
}

#[async_trait::async_trait]
pub trait DeviceStorage {
    async fn devices(&self) -> &HashMap<String, Device>;
}