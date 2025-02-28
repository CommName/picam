use crate::models::*;

pub mod sqlite;

pub struct Storage {
    pub users: Box<dyn UserStorage + Send + Sync>,
    pub camera_config: Box<dyn CameraConfigStorage + Send + Sync>
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

pub trait CameraConfigStorage {
    
}