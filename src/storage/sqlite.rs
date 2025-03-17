use std::sync::Arc;
use serde::Serialize;
use sqlx::{migrate::MigrateDatabase, sqlite::SqliteRow, Encode, Row, Sqlite, SqlitePool};
use super::{SimpleStorage, UserStorage};
use crate::models::*;
use log::*;

#[derive(Clone)]
pub struct SQLiteStorage {
    db: Arc<SqlitePool>
}

async fn create_database_if_it_does_not_exist(path: &str) {
    if !Sqlite::database_exists(path).await.unwrap_or(false) {
        info!("Creating database {}", path);
        match Sqlite::create_database(path).await {
            Ok(_) => println!("Create db success"),
            Err(error) => panic!("error: {}", error),
        }
    }

}

async fn run_migrations(db: &SqlitePool) {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let migrations = std::path::Path::new(&crate_dir).join("./migrations");
    let migration_results = sqlx::migrate::Migrator::new(migrations)
        .await
        .unwrap()
        .run(db)
        .await;
    match migration_results {
        Ok(_) => info!("Sucssesfully update database tables"),
        Err(error) => {
            panic!("Error running database migrations: {}", error);
        }
    }
}


impl SQLiteStorage {

    pub async fn new(path: &str) -> Self {
        create_database_if_it_does_not_exist(path).await;
        let db = Arc::new(SqlitePool::connect(path).await.unwrap());
        run_migrations(&db).await;

        Self {
            db: Arc::clone(&db)
        }
    }

}

fn user_from_row(r: SqliteRow) -> User {
    User {
        username: r.get::<String, &str>("username"),
        password: r.get::<String, &str>("password")
    }
}

#[async_trait::async_trait]
impl UserStorage for SQLiteStorage {
    
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
        let _ = sqlx::query("UPDATE users SET password=$1 where username=$2")
            .bind(&user.password)
            .bind(&user.username)
            .execute(self.db.as_ref())
            .await
            .map_err(|e| {
                error!("Error updating user into db {e:?}")
            });
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


async fn fetch_config(key: &str, db: &SqlitePool) -> Option<SqliteRow>{
    let row = sqlx::query("SELECT key, value FROM config WHERE key = $1")
        .bind(key)
        .fetch_one(db)
        .await;
    match row {
        Ok(sql_row) => {
            Some(sql_row)
        },
        Err(sqlx::Error::RowNotFound) => {
            None
        }
        Err(e) => {
            error!("Error fetching pipline config ({key}): {e:?}");
            None
        }
    }

}

async fn set_config<'a, T: sqlx::Type<Sqlite> + Encode<'a, Sqlite>>(key: &'a str, value: &'a T, db: &SqlitePool ) {
    if let Err(e) = sqlx::query("INSERT INTO config(key, value) VALUES ($1, $2)")
        .bind(key)
        .bind(value)
        .execute(db).await {
            error!("Error saving pipeline config {e:?}");
        }
}

async fn unset_config(key: &str, db: &SqlitePool) {
    if let Err(e) = sqlx::query("DELETE FROM config where key=$1")
        .bind(key)
        .execute(db).await {
            error!("Error removing old config {e:?}");
        }
}

async  fn update_paramter<'a, T: Serialize>(key: &'a str, value: &'a Option<T>, db: &SqlitePool) {
    unset_config(key, db).await;
    if let Some(value) = value {
        set_config(key, &serde_json::to_value(value).unwrap(), &db).await;
    }
}


const PIPELINE_CONFIG: &str = "pipeline_config";
#[async_trait::async_trait]
impl SimpleStorage<PipelineConfig> for SQLiteStorage {
    async fn get(&self) -> PipelineConfig {
        fetch_config(PIPELINE_CONFIG, self.db.as_ref())
            .await
            .map(|r| {
                let ret: sqlx::types::JsonValue = r.get("value");
                ret
                })
            .map(|j| serde_json::from_value(j).ok())
            .flatten().unwrap_or_default()
    }

    async fn set(&self, value: &PipelineConfig) {
        update_paramter(PIPELINE_CONFIG, &Some(value), self.db.as_ref()).await;
    }

}


const FILE_SINK_CONFIG: &str = "file_sink_config";
#[async_trait::async_trait]
impl SimpleStorage<FileSinkConfig> for SQLiteStorage {
    async fn get(&self) -> FileSinkConfig {
        fetch_config(FILE_SINK_CONFIG, self.db.as_ref())
        .await
        .map(|r| {
            let ret: sqlx::types::JsonValue = r.get("value");
            ret
            })
        .map(|j| serde_json::from_value(j).ok())
        .flatten().unwrap_or_default()
    }

    async fn set(&self, value: &FileSinkConfig) {
        update_paramter(FILE_SINK_CONFIG, &Some(value), self.db.as_ref()).await;
    }
}