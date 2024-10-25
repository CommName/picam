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
    moov: Data<& Arc<RwLock<Vec<Vec<u8>>>>>
) -> impl IntoResponse {
    let mut receiver = recv.subscribe();
    let moov = moov.0.read().unwrap().clone();
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

pub enum StateOfMp4 {
    moof,
    mdat,
    data,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GStreamer
    gstreamer::init()?;

    let (send, recv) = std::sync::mpsc::channel();
    let pipeline = video::build_gstreamer_pipline(send)?;


    let (tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(1024);
    let tx2 = tx.clone();

    let moov: Arc<RwLock<Vec<Vec<u8>>>> = Arc::new(RwLock::new(Vec::new()));
    let moov2 = Arc::clone(&moov);


    std::thread::spawn(move || {
        let mut pts = None;
        let mut vec = Vec::with_capacity(1024);
        let mut number_of_buffs = 0;
        {
            println!("Starting header");
            let mut moov = moov.write().unwrap();
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
        }

        println!("Started recv");
        let mut state = StateOfMp4::moof;
        while let Ok(buffer) = recv.recv() {

            println!("pts: {:?}", pts);
            pts = buffer.dts();

            let mapa = buffer.map_readable().unwrap();
            let mut slice = mapa.to_vec();
            //println!("Buffer {:?}", buffer);
            // moof 6d6f 6f66
            // mdat 6d64 6174

            if slice[4] == 0x6d && slice[5] == 0x6f && slice[6] == 0x6f && slice[7] == 0x66 {
                tx.send(vec.clone());
                // TODO send vector
                vec.clear();

            }

            vec.append(&mut slice);

        }
    });


    // Start playing
    pipeline.set_state(State::Playing)?;
    println!("Pipline started");

    // // Wait until error or EOS
    // let bus = pipeline.bus().unwrap();
    // for msg in bus.timed_pop_filtered(gstreamer::ClockTime::NONE, &[gstreamer::MessageType::Eos, gstreamer::MessageType::Error]) {
    //     match msg.view() {
    //         gstreamer::MessageView::Error(err) => {
    //             eprintln!("Error received from element {:?}: {}", msg.src().map(|s| s.name()), err.error());
    //             eprintln!("Debugging information: {:?}", err.debug());
    //             break;
    //         }
    //         gstreamer::MessageView::Eos(..) => {
    //             println!("End of stream reached.");
    //             break;
    //         }
    //         _ => (),
    //     }
    // }

    let app = Route::new()
        .at("/ws",
            get(ws)
            .data(tx2)
            .data(moov2)
        );

    let res = Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await;

    // Stop the pipeline
    pipeline.set_state(State::Null)?;

    

    Ok(())
}