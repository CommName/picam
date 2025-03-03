use std::sync::Arc;

use serde::{de::{value, DeserializeOwned}, Deserialize, Serialize};
use sqlx::{migrate::MigrateDatabase, sqlite::SqliteRow, Executor, Row, Sqlite, SqlitePool};

use super::{PipelineConfigStorage, UserStorage};
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

fn user_from_row(r: SqliteRow) -> User {
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

        res.into_iter().map(user_from_row).collect()
    }
    async fn get_user(&self, username: &str) -> Option<User> {
        Some(user_from_row(sqlx::query("SELECT username, password from users where username = $1;")
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


async fn fetch_config<T: DeserializeOwned>(key: &str, db: &SqlitePool) -> Option<T>{
    let row = sqlx::query("SELECT value FROM pipelineconfig WHERE key = $1")
        .bind(key)
        .fetch_one(db)
        .await;
    match row {
        Ok(sql_row) => {
            let json: String = sql_row.get("width");
            serde_json::from_str(json.as_str()).ok()
        },
        Err(sqlx::Error::RowNotFound) => {
            None
        }
        Err(e) => {
            error!("Error fetching pipline config (width): {e:?}");
            None
        }
    }
}

async fn set_config<T: Serialize>(key: &str, value: &T, db: &SqlitePool ) {
    sqlx::query("INSERT INTO pipelineconfig(key, value) VALUE($1, $2")
        .bind(key)
        .bind(&serde_json::to_string(value).unwrap())
        .execute(db);
}

async fn unset_config(key: &str, db: &SqlitePool) {
    sqlx::query("DELETE FROM pipelineconfig where key=$1")
        .bind(key)
        .execute(db);
}

async  fn update_paramter<T:Serialize>(key: &str, value: &Option<T>, db: &SqlitePool) {
    unset_config(key, db).await;
    if let Some(value) = value {
        set_config(key, value, &db).await;
    }
}

const WIDTH: &str = "width";
const HEIGHT: &str = "height";
const USE_BUILT_IN_ENCODER: &str = "use_cam_built_in_encoder";
const SOURCE: &str = "source";
#[async_trait::async_trait]
impl PipelineConfigStorage for SqliteCameraConfigStorage {

    async fn width(&self) -> Option<i32> {
        fetch_config(WIDTH, self.db.as_ref()).await
    }
    async fn height(&self) -> Option<i32> {
        fetch_config(HEIGHT, self.db.as_ref()).await
    }
    async fn use_cam_builtin_encoder(&self) -> Option<bool> {
        fetch_config(USE_BUILT_IN_ENCODER, self.db.as_ref()).await
    }
    async fn source(&self) -> Option<String> {
        fetch_config(SOURCE, self.db.as_ref()).await
    }

    async fn set_width(&self, value: Option<i32>) {
        update_paramter(WIDTH, &value, &self.db).await;
    }
    async fn set_height(&self, value: Option<i32>) {
        update_paramter(HEIGHT, &value, &self.db).await;
    }
    async fn set_use_cam_builtin_encoder(&self, value: Option<bool>) {
        update_paramter(USE_BUILT_IN_ENCODER, &value, &self.db).await;
    }
    async fn set_source(&self, value: Option<&str>) {
        update_paramter(SOURCE, &value, &self.db).await;
    }
}