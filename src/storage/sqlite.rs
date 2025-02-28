use std::sync::Arc;

use sqlx::{migrate::MigrateDatabase, sqlite::SqliteRow, Row, Sqlite, SqlitePool};

use super::{CameraConfigStorage, UserStorage};
use crate::models::*;
use log::*;


pub async fn SQLiteStorage(path: &str) -> super::Storage {
    if !Sqlite::database_exists(path).await.unwrap_or(false) {
        info!("Creating database {}", path);
        match Sqlite::create_database(path).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    }

    let db = Arc::new(SqlitePool::connect(path).await.unwrap());

    
    
    let users = SQLiteUserStorage {
        db: Arc::clone(&db)
    };
    let camera_config = SqliteCameraConfigStorage {
        db: Arc::clone(&db)
    };

    super::Storage { 
        users: Box::new(users),
        camera_config: Box::new(camera_config)
    }
}

pub struct SQLiteUserStorage {
    db: Arc<SqlitePool>
}

fn userFromRow(r: SqliteRow) -> User {
    User {
        username: r.get::<String, &str>("username"),
        password: r.get::<String, &str>("password")
    }
}

#[async_trait::async_trait]
impl UserStorage for SQLiteUserStorage {
    
    async fn get_users(&self ) -> Vec<User> {
        let res = sqlx::query("SELECT username, password from users;")
            .fetch_all(self.db.as_ref())
            .await
            .unwrap_or_else(|e| {
                error!("Failed to fetch users for sqlite db: {e:?}");
                Vec::new()}
            );

        res.into_iter().map(userFromRow).collect()
    }
    async fn get_user(&self, username: &str) -> Option<User> {
        Some(userFromRow(sqlx::query("SELECT username, password from users where username = $1;")
            .bind(username)
            .fetch_one(self.db.as_ref())
            .await
            .ok()?))
    }

    async fn number_of_users(&self) -> usize {
        let res: i32 = sqlx::query_scalar("SELECT COUNT(*) FROM users;")
            .fetch_one(self.db.as_ref()) 
            .await
            .unwrap_or_default();

        res as usize
    }
    async fn update_user(&self, user: &User) {
        todo!()
    }
    async fn create_user(&self, user: &User) {
        let _ = sqlx::query("INSERT INTO users (username, password) values ($1, $2)")
            .bind(&user.username)
            .bind(&user.password)
            .execute(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("Error inserting user into db {e:?}")
            });
    }
    async fn delete_user(&self, username: &str) {
        let _ = sqlx::query("DELETE FROM users WHERE username=$1")
            .bind(username)
            .execute(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("Error deleting user: {e:?}")
            });
    }
}

pub struct SqliteCameraConfigStorage {
    db: Arc<SqlitePool>
}

impl CameraConfigStorage for SqliteCameraConfigStorage {

}