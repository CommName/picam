use std::sync::Arc;

use poem::session::Session;
use sha3::{Digest, Sha3_256};

use crate::models::User;
use crate::storage::Storage;
use crate::api_handlers::AuthError;


pub async fn init_user(admin: User,  storage: &Arc<Storage>) {
    let number_of_users = storage.users.number_of_users().await;
    if number_of_users != 0 {
        return;
    }

    storage.users.create_user(&admin).await;
}

pub async fn auth_user(user: User, storage: &Arc<Storage>) -> Result<User, AuthError> {
    let db_user = storage.users.get_user(&user.username).await.ok_or_else(|| AuthError::UserMissing)?;

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