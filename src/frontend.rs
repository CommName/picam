

use std::sync::Arc;

use poem::http::Method;
use poem::session::Session;
use poem::web::{Data, Form};
use poem::{Endpoint, FromRequest, Response};
use poem::endpoint::StaticFilesEndpoint;
use tokio::sync::RwLock;

use crate::api_handlers::AuthUser;
use crate::models::User;
use crate::storage::Storage;


pub struct Frontend {
    index: StaticFilesEndpoint,
    login_page: StaticFilesEndpoint,
    init_page: StaticFilesEndpoint,
    initialize: RwLock<bool>,
}

impl Frontend {
    pub async fn new(storage: Arc<Storage>) -> Self {
        let index = StaticFilesEndpoint::new("./frontend/index.html");
        let login_page = StaticFilesEndpoint::new("./frontend/login.html");
        let init_page = StaticFilesEndpoint::new("./frontend/init.html");
        let initialize = RwLock::new(storage.users.number_of_users().await > 0);

        Self {
            index,
            login_page,
            init_page,
            initialize,
        }
    }
}

impl Endpoint for Frontend {
    type Output = Response;
    async fn call(&self, req: poem::Request) -> poem::Result<Self::Output> {
        let (mut req, mut body) = req.split();
        let Data(storage):  Data<&Arc<Storage>> = Data::from_request_without_body(&req).await.unwrap();
        if req.method() == Method::GET {
            let init = self.initialize.read().await.clone();
            if init {
                let auth_user = AuthUser::from_request(&req, &mut body).await;
                if auth_user.is_ok() {
                    self.index.call(req).await
                } else {
                    self.login_page.call(req).await
                }
            } else {
                self.init_page.call(req).await
            }

        } else if req.method() == Method::POST {
            let user= Form::<User>::from_request(&req,&mut body).await.unwrap();            
            let session =  <&Session as FromRequest>::from_request(&req,&mut body).await?;

            let init = self.initialize.read().await.clone();
            if init {
                let auth = crate::users::auth_user(user.0, &storage).await;
                if let Ok(user) = auth {
                    crate::users::set_session(&user, session);
                    req.set_method(Method::GET);
                    self.index.call(req).await
                } else {
                    req.set_method(Method::GET);
                    self.login_page.call(req).await
                }

            } else {
                let mut init = self.initialize.write().await;
                crate::users::init_user(user.0.clone(), &storage).await;
                *init = true;
                crate::users::set_session(&user, session);
                req.set_method(Method::GET);
                
                self.index.call(req).await
            }
        } else {
            // 404 Error !?
            self.index.call(req).await
        }
        
    }

}