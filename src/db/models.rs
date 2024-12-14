use diesel::prelude::*;
use poem_openapi::Object;

#[derive(Object, Queryable, Selectable, Insertable, Debug)]
#[diesel(table_name = super::schema::users)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct User {
    pub username: String,
    pub password: String
}