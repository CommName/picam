use diesel::prelude::*;
use models::User;
use schema::users;
pub mod models;
pub mod schema;

use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
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
    diesel::update(users.find(user.username))
        .set(password.eq(user.password))
        .execute(con)
        .unwrap();
}

pub fn create_user(con: &mut SqliteConnection, user: &User) {
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