use std::sync::Arc;

use config::Config;
use futures_util::{SinkExt, StreamExt};
use gstreamer::{prelude::*, ClockTime, State};
use poem::endpoint::StaticFilesEndpoint;
use poem::http::Method;
use poem::listener::TcpListener;
use poem::middleware::Cors;
use poem::web::websocket::Message;
use poem::web::Data;
use poem::{get, EndpointExt, IntoResponse, Route, Server};
use poem::{handler, web::websocket::WebSocket};
use poem_openapi::OpenApiService;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;

mod api_handlers;
mod config;
mod sys;
mod video;
mod file_sink;
mod db;

#[handler]
fn ws(
    ws: WebSocket,
    recv: Data<& tokio::sync::broadcast::Sender<Arc<ParsedBuffer>>>,
    Data(moov): Data<&Arc<Vec<Vec<u8>>>>
) -> impl IntoResponse {
    let mut receiver = recv.subscribe();

    let moov = Arc::clone(moov);
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        for pack in moov.iter() {
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
                        if let Err(_) = if iframe_sent {
                            sink.send(poem::web::websocket::Message::binary(msg.data.clone())).await
                        } else {
                            if msg.key_frame {
                                iframe_sent = true;
                                sink.send(poem::web::websocket::Message::binary(msg.data.clone())).await
                            } else {
                                continue;
                            }
                        } {
                            // Error sending a message
                            let _ = sink.close();
                            return;
                        }
                        
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


pub async  fn get_moov_header(mut recv: Receiver<Arc<ParsedBuffer>>) -> Arc<Vec<Vec<u8>>> {
    let mut moov: Vec<Vec<u8>> = Vec::new();
    let mut number_of_buffs = 0;
    while number_of_buffs < 2 {
        if let Ok(buffer) = recv.recv().await {
            moov.push(buffer.data.clone());
            number_of_buffs += 1;
        } else {
            break;
        }
    }
    Arc::new(moov)
}

pub struct ParsedBuffer {
    data: Vec<u8>,
    key_frame: bool,
    timestamp: Option<ClockTime>
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env();
    println!("Config file: {config:?}");

    let mut connection = db::establish_connection();
    db::update_db_migrations(&mut connection);
    let connection = Arc::new(Mutex::new(connection));
    
    
    // Initialize GStreamer
    let devices = sys::Device::devices();
    println!("Devices detected: {devices:?}");
    gstreamer::init()?;
    println!("Gstreamer initizalized");

    let (tx, rx) = tokio::sync::broadcast::channel::<Arc<ParsedBuffer>>(1024);
    let pipeline = video::build_gstreamer_pipline(tx.clone(), config)?;
    println!("Pipeline created");


    let tx2 = tx.clone();

    
    
    // Start playing
    pipeline.set_state(State::Playing)?;
    println!("Pipline started");
    
    let moov = get_moov_header(rx).await;

    std::thread::spawn(move || {
    });

    let moov2 = Arc::clone(&moov);
    let file_sink_subscirber = tx2.subscribe();
    tokio::spawn(async move {
        file_sink::file_saver(file_sink_subscirber, moov2).await;
    });

    let cors = Cors::new()
        .allow_method(Method::GET)
        .allow_method(Method::POST)
        .allow_origin_regex("*");

    let api_service =
        OpenApiService::new(api_handlers::Api, "PICam", "0.1").server("http://localhost:3000");

    println!("Starting server");

    let app = Route::new()
        .nest("/", StaticFilesEndpoint::new("./frontend/index.html"))
        .nest("/pico.css", StaticFilesEndpoint::new("./frontend/pico.css"))
        .at("/ws",
            get(ws)
            .data(tx2)
            .data(moov)
        )
        .nest("/api", api_service)
            .data(connection)
            .with(cors);

    let _ = Server::new(TcpListener::bind("0.0.0.0:8080"))
        .run(app)
        .await;

    // Stop the pipeline
    pipeline.set_state(State::Null)?;

    

    Ok(())
}