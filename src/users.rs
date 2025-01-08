use std::sync::Arc;

use diesel::SqliteConnection;
use tokio::sync::Mutex;

use crate::db::models::User;
use crate::db;


pub async fn init_user(admin: User,  db: &Arc<Mutex<SqliteConnection>>) {
    let mut db = db.lock().await;
    let number_of_users = db::number_of_users(&mut db);
    if number_of_users != 0 {
        return;
    }

    db::create_user(&mut db, admin);
}