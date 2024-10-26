use std::env::var;
use std::io::Write;
use std::ops::Deref;
use std::sync::Arc;

use byte_slice_cast::AsByteSlice;
use futures_util::{SinkExt, StreamExt};
use glib::DateTime;
use gstreamer::{prelude::*, FlowSuccess, Message, Sample};
use gstreamer::{ElementFactory, Pipeline, State, StateChangeSuccess};
use gstreamer_app::AppSink;
use poem::listener::TcpListener;
use poem::web::Data;
use poem::{get, EndpointExt, IntoResponse, Route, Server};
use poem::{handler, web::websocket::{WebSocket, WebSocketStream} };
use std::sync::RwLock;


mod video;
mod file_sink;

#[handler]
fn ws(
    ws: WebSocket,
    recv: Data<& tokio::sync::broadcast::Sender<Vec<u8>>>,
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
        while let Ok(msg) = receiver.recv().await {
            if sink.send(poem::web::websocket::Message::binary(msg)).await.is_err() {
                println!("Error sending message");
                break;
            }
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GStreamer
    gstreamer::init()?;

    let (send, recv) = std::sync::mpsc::channel();
    let pipeline = video::build_gstreamer_pipline(send)?;


    let (tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(1024);
    let tx2 = tx.clone();

    
    
    // Start playing
    pipeline.set_state(State::Playing)?;
    println!("Pipline started");
    
    let mut moov: Vec<Vec<u8>> = Vec::new();
    println!("Saving header");
    let mut number_of_buffs = 0;
    while number_of_buffs < 2 {
        if let Ok(buffer) = recv.recv() {
            let mapa = buffer.map_readable().unwrap();
            let slice = mapa.to_vec();
            moov.push(slice);
            number_of_buffs += 1;
        } else {
            break;
        }
    }
    let moov = Arc::new(moov);


    std::thread::spawn(move || {
        let mut pts = None;
        let mut vec = Vec::with_capacity(1024);


        println!("Started recv");
        while let Ok(buffer) = recv.recv() {

            println!("pts: {:?}", pts);
            pts = buffer.dts();

            let mapa = buffer.map_readable().unwrap();
            let mut slice = mapa.to_vec();
            // moof 6d6f 6f66
            if slice[4] == 0x6d && slice[5] == 0x6f && slice[6] == 0x6f && slice[7] == 0x66 {
                tx.send(vec.clone());
                vec.clear();

            }

            vec.append(&mut slice);

        }
    });



    let app = Route::new()
        .at("/ws",
            get(ws)
            .data(tx2)
            .data(moov)
        );

    let res = Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await;

    // Stop the pipeline
    pipeline.set_state(State::Null)?;

    

    Ok(())
}