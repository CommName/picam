use std::sync::Arc;

use diesel::SqliteConnection;
use poem::session::Session;
use sha3::{Digest, Sha3_256};
use tokio::sync::Mutex;

use crate::db::models::User;
use crate::db;
use crate::api_handlers::AuthError;


pub async fn init_user(admin: User,  db: &Arc<Mutex<SqliteConnection>>) {
    let mut db = db.lock().await;
    let number_of_users = db::number_of_users(&mut db);
    if number_of_users != 0 {
        return;
    }

    db::create_user(&mut db, admin);
}

pub async fn auth_user(user: User, db: &Arc<Mutex<SqliteConnection>>) -> Result<User, AuthError> {
    let mut db = db.lock().await;
    let db_user = db::get_user(&mut db, &user.username)
        .ok_or_else(|| AuthError::UserMissing)?;
    drop(db);

    let password_hash =  Sha3_256::digest(user.password);
    let password_hash = format!("{:x}", password_hash);

    if db_user.password == password_hash {
        Ok(db_user)
    } else {
        Err(AuthError::InccorectPassword)
    }
}

pub fn set_session(user: &User, session: &Session) {
    session.set("user", user.username.clone());
}