use std::time::Duration;

use diesel::prelude::*;

use models::User;
use schema::users;
pub mod models;
pub mod schema;

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use sha3::{Digest, Sha3_256};
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub fn establish_connection() -> SqliteConnection {

    let database_url = String::from("./picam.db"); //TODO: change to appdata relative path
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub fn update_db_migrations(con: &mut SqliteConnection) {
    con.run_pending_migrations(MIGRATIONS)
    .unwrap();
}


pub fn get_users(con: &mut SqliteConnection) -> Vec<User> {
    use self::schema::users::dsl::*;
    let result = users
        .select(User::as_select())
        .load(con)
        .unwrap(); // TODO handle error

    result
}

pub fn get_user(con: &mut SqliteConnection, user: &str) -> Option<User> {
    use self::schema::users::dsl::*;
    users.find(user)
        .first(con)
        .optional()
        .unwrap()
}

pub fn number_of_users(con: &mut SqliteConnection) -> usize {
    use self::schema::users::dsl::*;
    users.count()
        .execute(con)
        .unwrap()
}

pub fn update_user(con: &mut SqliteConnection, user: User) {
    use self::schema::users::dsl::*;
    let password_hash =  Sha3_256::digest(user.password);
    let password_hash = format!("{:x}", password_hash);

    diesel::update(users.find(user.username))
        .set(password.eq(password_hash))
        .execute(con)
        .unwrap();
}

pub fn create_user(con: &mut SqliteConnection, user: User) {
    let password_hash =  Sha3_256::digest(user.password);
    let password_hash = format!("{:x}", password_hash);
    let user = User {
        password: password_hash,
        ..user
    };

    diesel::insert_into(users::table)
        .values(user)
        .execute(con)
        .unwrap();

}

pub fn delete_user(con: &mut SqliteConnection, user: String) {
    use self::schema::users::dsl::*;
    diesel::delete(users.find(user))
        .execute(con);
}