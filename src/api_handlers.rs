use std::{path::PathBuf, str::FromStr, sync::Arc};

use diesel::SqliteConnection;
use poem::{session::Session, web, FromRequest};
use poem_openapi::{param::Path, payload::{Binary, Json, Response}, ApiResponse, Object, OpenApi, SecurityScheme};
use tokio::sync::Mutex;
use crate::db::{self, models::User};

type Result<T> = std::result::Result<T, Error>;

#[derive(ApiResponse)]
pub enum Error {
    #[oai (status="404")]
    NotFound(Json<ErrorMessage>),
    #[oai (status="400")]
    BadRequest(Json<ErrorMessage>),
    #[oai (status="401")]
    AuthError(Json<ErrorMessage>)
}

#[derive(Object)]
pub struct ErrorMessage {
    error: String
}

#[derive(Clone, Object)]
struct PiCamClaim {
    user: String
}

#[derive(Debug)]
pub enum AuthError {
    UserMissing,
    InccorectPassword
}

pub struct  AuthUser{
    username: String
}

impl<'a> FromRequest<'a> for AuthUser {
    async fn from_request(
            req: &'a poem::Request,
            body: &mut web::RequestBody,
        ) -> poem::Result<Self> {
        let session: &Session  = <&Session as FromRequest>::from_request(req, body).await?;

        let username: String = session.get("user")
            .ok_or_else(|| Error::AuthError(Json(ErrorMessage{
                error: "Unauthenticated user".to_string()
            })))?;

        Ok(AuthUser {
            username
        })
    }
}


impl From<AuthError> for Error {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::UserMissing => Error::BadRequest(Json(ErrorMessage {
                 error: String::from("Invalid username") 
            })),
            AuthError::InccorectPassword => Error::BadRequest(Json(ErrorMessage {
                error: String::from("Invalid password") 
           }))
        }
    }
}

pub struct Api;

#[OpenApi]
impl Api {
    /// Hello world
    #[oai(path = "/recordings", method = "get")]
    async fn list_recordings(&self) -> Json<Vec<String>> {
        let mut recordings = Vec::new();

        let mut dir = tokio::fs::read_dir(crate::file_sink::APP_DATA_PATH).await
            .unwrap();

        while let Ok(Some(f)) = dir.next_entry().await {
            let path = f.path();
            if path.extension().map(|e| e == "mp4").unwrap_or(false) {
                recordings.push(path.file_name().unwrap().to_str().unwrap().to_string());
            }
        }
        
        Json(recordings)
    }


    #[oai(path = "/recordings/:recording", method = "get")]
    async fn download_recordings(&self, Path(recording): Path<String>) -> Response<Binary<Vec<u8>>> {

        let path = PathBuf::from_str(&(crate::file_sink::APP_DATA_PATH.to_string() + "/" + &recording))
            .unwrap(); // TODO handle this error

        if path.parent().map(|p| p.as_os_str() != crate::file_sink::APP_DATA_PATH).unwrap_or(true) {
            println!("Tempered path from user side");
        }
        // TODO check if file exists

        let content = tokio::fs::read(path).await
            .unwrap(); // TODO handle this error

        Response::new(Binary(content)).header(poem::http::header::CONTENT_TYPE, "video/mp4")
    }


    #[oai(path = "/users/init", method = "post")]
    async  fn init_user(&self, Json(admin): Json<User>,  db: web::Data<&Arc<Mutex<SqliteConnection>>>) {
        crate::users::init_user(admin, db.0).await;
    }


    #[oai(path = "/users", method = "get")]
    async fn get_users(&self, db: web::Data<&Arc<Mutex<SqliteConnection>>>, _user: AuthUser) -> Json<Vec<String>> {
        let mut db = db.lock().await;
        let users = db::get_users(&mut db)
            .into_iter()
            .map(|u| u.username)
            .collect();
        Json(users)
    }

    #[oai(path = "/auth", method = "post")]
    async  fn auth(
        &self, 
        Json(user): Json<User>, 
        db: web::Data<&Arc<Mutex<SqliteConnection>>>, 
        session: &Session,
    ) -> Result<Json<String>> {
        let user = crate::users::auth_user(user, &db).await?;
        crate::users::set_session(&user, session);

        Ok(Json(user.username))
    }

    #[oai(path = "/users/update", method = "post")]
    async  fn update_user(&self, Json(user): Json<User>, db: web::Data<&Arc<Mutex<SqliteConnection>>>) {
        let mut db = db.lock().await;
        db::update_user(&mut db, user);
    }

    #[oai(path = "/users/register", method = "post")]
    async  fn register_user(&self,  Json(user): Json<User>, db: web::Data<&Arc<Mutex<SqliteConnection>>>) {
        let mut db = db.lock().await;
        db::create_user(&mut db, user);
    }


    #[oai(path = "/users/delete", method = "delete")]
    async  fn delete_user(&self, Json(user): Json<String>, db: web::Data<&Arc<Mutex<SqliteConnection>>>) {
        let mut db = db.lock().await;
        db::delete_user(&mut db, user);
    }

}
