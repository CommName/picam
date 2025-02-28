use poem_openapi::Object;
use serde::{Deserialize, Serialize};


#[derive(Object, Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub username: String,
    pub password: String
}