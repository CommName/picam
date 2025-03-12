use std::sync::Arc;
use std::time::Duration;
use api_handlers::AuthUser;
use config::Config;
use file_sink::FileSinkConfig;
use futures_util::{SinkExt, StreamExt};
use gstreamer::glib::ControlFlow;
use gstreamer::{prelude::*, ClockTime, MessageView, State};
use poem::endpoint::StaticFilesEndpoint;
use poem::http::Method;
use poem::listener::TcpListener;
use poem::web::{cookie::CookieKey, websocket::{Message, WebSocket}, Data, Form};
use poem::{get, middleware::Cors, Endpoint, EndpointExt, FromRequest, IntoResponse, Response, Route, Server, handler};
use poem_openapi::OpenApiService;
use storage::Storage;
use tokio::sync::broadcast::Sender;
use tokio::sync::RwLock;
use poem::session::{CookieConfig, CookieSession, Session};
use log::*;
use models::*;

mod api_handlers;
mod users;
mod config;
mod video;
mod file_sink;
mod storage;
mod models;

#[handler]
fn ws(
    ws: WebSocket,
    recv: Data<& tokio::sync::broadcast::Sender<Arc<ParsedBuffer>>>,
    Data(moov): Data<&Arc<RwLock<Vec<Vec<u8>>>>>
) -> impl IntoResponse {
    let mut receiver = recv.subscribe();

    let moov = Arc::clone(moov);
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        for pack in moov.read().await.iter() {
            let data = pack.clone();
            if sink.send(poem::web::websocket::Message::binary(data)).await.is_err() {
                break;
            };
        }
        drop(moov);
        // Start with IFrame
        let mut iframe_sent = false;
        loop {
            tokio::select! {
                msg = receiver.recv() => {
                    if let Ok(msg) = msg {
                        match msg.message_type {
                            MessageType::FirstFrame => {
                                let _ = sink.send(Message::Close(None)).await; // TODO research and specify reason
                                let _ = sink.close().await;
                                return;
                            },
                            MessageType::KeyFrame if !iframe_sent => {
                                iframe_sent = true;
                                let _ = sink.send(poem::web::websocket::Message::binary(msg.data.clone())).await;
                            },
                            _ if iframe_sent => {
                                let _ = sink.send(poem::web::websocket::Message::binary(msg.data.clone())).await;
                            },
                            _ => {
                                continue;
                            }
                        };
                    }
            },
                msg = stream.next() => {
                    if let Some(Ok(msg)) = msg {
                        match msg {
                            Message::Ping(bytes) => {
                                let _ = sink.send(Message::Pong(bytes)).await;
                            },
                            Message::Close(status_code) => {
                                let _ = sink.send(Message::Close(status_code));
                                let _ = sink.close();
                                return;
                            },
                            _ => {}
                        }
                    }    
                }
            }
        }


    })
}

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
                let auth = users::auth_user(user.0, &storage).await;
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
                users::init_user(user.0.clone(), &storage).await;
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


#[derive(PartialEq, Debug)]
pub enum MessageType {
    KeyFrame,
    FirstFrame,
    MoovPacket,
    Fragment
}

pub struct ParsedBuffer {
    data: Vec<u8>,
    message_type: MessageType,
    timestamp: Option<ClockTime>
}

pub async fn pipeline_watchdog(storage: Arc<Storage>, tx: Sender<Arc<ParsedBuffer>>) {

    gstreamer::init().unwrap();

    loop {
        let config = storage.camera_config.pipeline_config().await;
        let devices = storage.devices.devices().await;
        let config = video::Config::find_optimal_settings(devices, config);

        info!("Starting new pipline with config: {config:?}");

        let pipeline = video::build_gstreamer_pipline(tx.clone(), &config);

        match pipeline {
            Ok(pipeline) => {
                let pipeline_weak = pipeline.downgrade();
                let Some(bus) = pipeline.bus() else {
                    continue;
                };
                let main_loop = glib::MainLoop::new(None, false);
                let main_loop_ref = main_loop.clone();

                let _ = bus.add_watch(move |_, message| {
                    let main_loop = &main_loop_ref;
                    debug!("New messaged on the buss: {message:?}");
                    match message.view() {
                        MessageView::Eos(_) => {
                            error!("End of stream reached!, Restarting pipeline");
                            main_loop.quit();
                        },
                        MessageView::Error(err) => {
                            warn!("Pipeline error: {err:?}");
                            main_loop.quit();
                        },
                        MessageView::StateChanged(statechange) => {
                            match pipeline_weak.upgrade() {
                                Some(pipeline) => {
                                    let prev = statechange.old();
                                    let curr = statechange.current();
                                    info!("State changed from {prev:?} to {curr:?}");
                                    match curr {
                                        State::Null => {
                                            main_loop.quit();
                                            return  ControlFlow::Break;
                                        },
                                        State::Paused => {
                                            if prev == State::Playing {
                                                warn!("Pipline went from Playing state to Paused, restarting pipline");
                                                if let Err(e) = pipeline.set_state(State::Playing) {
                                                    error!("Error when restarting pipeline {e:?}");
                                                    return ControlFlow::Break;
                                                }
                                            }
                                        },
                                        State::Ready => {
                                            if prev == State::Playing || prev== State::Paused {
                                             warn!("Pipline went from Playing/Paused to Ready state");
                                                // pipeline.set_state(State::Playing);
                                                main_loop.quit();
                                                return  ControlFlow::Break;
                                            }
                                        },
                                        State::Playing => {
                                            info!("Pipline is playing");
                                        },
                                        State::VoidPending => {
                                            info!("Void pending");
                                        }
                                    }
                                },
                                None => {
                                    main_loop.quit();
                                    return ControlFlow::Break;
                                }

                            }

                        },
                        _ => {

                        }
                    }
                    ControlFlow::Continue
                });

                info!("Starting pipline");
                if let Err(e) = pipeline.set_state(State::Playing) {
                    error!("Failed to start pipline {e:?}");
                }
                main_loop.run();
                let _ = pipeline.set_state(State::Null);
                std::thread::sleep(Duration::from_secs(15));
            },
            Err(e) => {
                error!("Error creating pipline: {e:?}");
                break;
            }
        }
    }
    error!("Exiting watchdog!");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();

    env_logger::init();
    let storage = Arc::new(Storage::new_sqlite(&config.app_data).await);

    let (tx, _) = tokio::sync::broadcast::channel::<Arc<ParsedBuffer>>(1024); 
    
    let moov: Arc<RwLock<Vec<Vec<u8>>>> = Arc::new(RwLock::new(Vec::new()));

    let moov2 = Arc::clone(&moov);
    let file_sink_subscirber = tx.subscribe();
    tokio::spawn(async move {
        video::init_moov_header(file_sink_subscirber, moov2).await;
    });

    let tx2 = tx.clone();
    let storage2 = Arc::clone(&storage);
    tokio::task::spawn(async move {
        pipeline_watchdog(storage2, tx).await;
    });

    let moov2 = Arc::clone(&moov);
    let file_sink_subscirber = tx2.subscribe();
    let (config_tx, config_rx) = tokio::sync::broadcast::channel(5);
    tokio::spawn(async move {
        file_sink::file_saver(file_sink_subscirber, moov2, config_rx).await;
    });

    let _ = config_tx.send(Arc::new(FileSinkConfig::default()));

    let cors = Cors::new()
        .allow_method(Method::GET)
        .allow_method(Method::POST)
        .allow_origin_regex("*");

    let api_service =
        OpenApiService::new(api_handlers::Api, "PICam", "0.1").server("http://localhost:8080");

    println!("Starting server");

    let app = Route::new()
        .nest("/", Frontend::new(Arc::clone(&storage)).await)
        .nest("/pico.css", StaticFilesEndpoint::new("./frontend/pico.css"))
        .nest("/picam.css", StaticFilesEndpoint::new("./frontend/picam.css"))
        .at("/ws",
            get(ws)
            .data(tx2)
            .data(moov)
        )
        .nest("/api", api_service)
            .data(Arc::clone(&storage))
            .with(CookieSession::new(CookieConfig::signed(CookieKey::generate())))
            .with(cors);

    info!("Listening on: {}", config.bind);
    if let Err(e) = Server::new(TcpListener::bind(config.bind))
        .run(app)
        .await {
            error!("Error starting server: {e:?}")
    }

    Ok(())
}