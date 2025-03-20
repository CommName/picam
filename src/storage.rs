use std::collections::HashMap;
use tokio::sync::broadcast::Receiver;

use crate::{config::Config, models::*};

mod sqlite;
mod memory;
mod simple_observable;

pub struct Storage {
    pub users: Box<dyn UserStorage + Send + Sync>,
    pub camera_config: Box<dyn ObservableStorage<PipelineConfig> + Send + Sync>,
    pub file_config: Box<dyn ObservableStorage<FileSinkConfig> + Send + Sync>,
    pub devices: Box<dyn DeviceStorage + Send + Sync>,
    pub config: Config

}

impl Storage {

    pub async fn new_sqlite(sqlite_path: &str) -> Self {   
        use simple_observable::SimpleObservable;
        let sqlite_storage = sqlite::SQLiteStorage::new(&sqlite_path).await;

        let devices = Box::new(memory::MemoryDeviceStorage::default());
        let config = Config::from_env();

        Self {
            users: Box::new(sqlite_storage.clone()),
            file_config: Box::new(SimpleObservable::new(sqlite_storage.clone())),
            camera_config: Box::new(SimpleObservable::new(sqlite_storage)),
            devices,
            config
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

#[async_trait::async_trait]
pub trait Observable<T> {
    async fn subscribe(&self) -> Receiver<T>;
}

pub trait ObservableStorage<T> : Observable<T> + SimpleStorage<T> {   
}
