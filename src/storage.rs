use std::collections::HashMap;
use crate::models::*;

pub mod sqlite;
pub mod memory;

pub struct Storage {
    pub users: Box<dyn UserStorage + Send + Sync>,
    pub camera_config: Box<dyn SimpleStorage<PipelineConfig> + Send + Sync>,
    pub devices: Box<dyn DeviceStorage + Send + Sync>

}

impl Storage {

    pub async fn new_sqlite(sqlite_path: &str) -> Self {   
        let sqlite_storage = sqlite::SQLiteStorage::new(&sqlite_path).await;

        let devices = Box::new(memory::MemoryDeviceStorage::default());

        Self {
            users: Box::new(sqlite_storage.clone()),
            camera_config: Box::new(sqlite_storage),
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


#[async_trait::async_trait]
pub trait SimpleStorage<T> {
    async fn get(&self) -> T;
    async fn set(&self, value: &T);
}

#[async_trait::async_trait]
pub trait DeviceStorage {
    async fn devices(&self) -> &HashMap<String, Device>;
}