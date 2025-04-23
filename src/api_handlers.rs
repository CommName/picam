use std::{path::PathBuf, str::FromStr, sync::Arc};

use poem::{session::Session, web, FromRequest};
use poem_openapi::{param::Path, payload::{Binary, Json, Response}, ApiResponse, Object, OpenApi};
use crate::{models::*, storage::*};

type Result<T> = std::result::Result<T, Error>;

#[derive(ApiResponse)]
pub enum Error {
    #[oai (status="404")]
    NotFound(Json<ErrorMessage>),
    #[oai (status="400")]
    BadRequest(Json<ErrorMessage>),
    #[oai (status="401")]
    AuthError(Json<ErrorMessage>),
    #[oai (status=505)]
    ServerError(Json<ErrorMessage>)
}

impl Error {
    pub fn not_found(error: String) -> Self {
        Self::NotFound(Json(ErrorMessage{
            error
        }))
    }

    pub fn server_error(error: String) -> Self {
        Self::ServerError(Json(ErrorMessage{
            error
        }))
    }
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

pub struct AuthUser{
    _username: String
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
            _username: username
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
    async fn list_recordings(&self, storage: web::Data<&Arc<Storage>>) -> Result<Json<Vec<String>>> {
        let mut recordings = Vec::new();

        let mut dir = tokio::fs::read_dir(storage.config.app_data.clone()).await
            .map_err(|e| Error::server_error(format!("Failed to create path to app_data dir {e:?}")))?;

        while let Ok(Some(f)) = dir.next_entry().await {
            let path = f.path();
            if path.extension().map(|e| e == "mp4").unwrap_or(false) {
                recordings.push(path.file_name().unwrap_or_default().to_str().unwrap_or_default().to_string());
            }
        }
        
        Ok(Json(recordings))
    }


    #[oai(path = "/recordings/:recording", method = "get")]
    async fn download_recordings(&self, Path(recording): Path<String>, storage: web::Data<&Arc<Storage>>) -> Result<Response<Binary<Vec<u8>>>> {

        let path = PathBuf::from_str(&(storage.config.app_data.clone() + "/" + &recording))
            .map_err(|e| Error::server_error(format!("Failed to create path to app_data dir {e:?}")))?;

        if path.parent().map(|p| p.as_os_str() != storage.config.app_data.as_str()).unwrap_or(true) {
            println!("Tempered path from user side");
        }
        if !tokio::fs::try_exists(&path).await.map_err(|e| Error::server_error(format!("Failed to read content of recording {e:?}")))? {
            Err(Error::not_found("Recording not found".to_string()))?;
        }

        let content = tokio::fs::read(&path).await
            .map_err(|e| Error::server_error(format!("Failed to read content of recording {e:?}")))?; // TODO handle this error

        Ok(Response::new(Binary(content)).header(poem::http::header::CONTENT_TYPE, "video/mp4"))
    }


    #[oai(path= "/recordings/config", method ="get")]
    async fn get_file_config(&self,  storage: web::Data<&Arc<Storage>>) -> Json<FileSinkConfig> {
        let config = storage.file_config.get().await;
        Json(config)
    }
    #[oai(path= "/recordings/config", method ="post")]
    async fn set_file_config(&self, config: Json<FileSinkConfig>,  storage: web::Data<&Arc<Storage>>) {
        storage.file_config.set(&config).await;
    }

    #[oai(path = "/users/init", method = "post")]
    async  fn init_user(&self, Json(admin): Json<User>,  storage: web::Data<&Arc<Storage>>) {
        crate::users::init_user(admin, &storage).await;
    }


    #[oai(path = "/users", method = "get")]
    async fn get_users(&self, storage: web::Data<&Arc<Storage>>, _user: AuthUser) -> Json<Vec<String>> {
        let users = storage.users.get_users().await
            .into_iter()
            .map(|u| u.username)
            .collect();
        Json(users)
    }

    #[oai(path = "/auth", method = "post")]
    async  fn auth(
        &self, 
        Json(user): Json<User>, 
        storage: web::Data<&Arc<Storage>>, 
        session: &Session,
    ) -> Result<Json<String>> {
        let user = crate::users::auth_user(user, &storage).await?;
        crate::users::set_session(&user, session);

        Ok(Json(user.username))
    }

    #[oai(path = "/users/update", method = "post")]
    async  fn update_user(&self, Json(user): Json<User>, storage: web::Data<&Arc<Storage>>) {
        storage.users.update_user(&user).await;
    }

    #[oai(path = "/users/register", method = "post")]
    async  fn register_user(&self,  Json(user): Json<User>, storage: web::Data<&Arc<Storage>>) {
        storage.users.create_user(&user).await;
    }


    #[oai(path = "/users/delete", method = "delete")]
    async  fn delete_user(&self, Json(user): Json<String>, storage: web::Data<&Arc<Storage>>) {
        storage.users.delete_user(&user).await;
    }

    #[oai(path= "/pipeline/config", method ="get")]
    async fn get_config(&self,  storage: web::Data<&Arc<Storage>>) -> Json<PipelineConfig> {
        let config = storage.camera_config.get().await;
        Json(config)
    }

    #[oai(path= "/pipeline/config", method ="post")]
    async fn set_config(&self, config: Json<PipelineConfig>,  storage: web::Data<&Arc<Storage>>) {
        storage.camera_config.set(&config).await;
    }


}
