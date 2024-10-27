use std::sync::mpsc::Receiver;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use gstreamer::{prelude::*, Buffer, BufferFlags, State};
use poem::listener::TcpListener;
use poem::web::Data;
use poem::{get, EndpointExt, IntoResponse, Route, Server};
use poem::{handler, web::websocket::WebSocket};

mod video;
mod file_sink;

#[handler]
fn ws(
    ws: WebSocket,
    recv: Data<& tokio::sync::broadcast::Sender<Arc<ParsedBuffer>>>,
    Data(moov): Data<&Arc<Vec<Vec<u8>>>>
) -> impl IntoResponse {
    let mut receiver = recv.subscribe();

    let moov = Arc::clone(moov);
    ws.on_upgrade(move |socket| async move {
        let (mut sink, _) = socket.split();
        for pack in moov.iter() {
            let data = pack.clone();
            if sink.send(poem::web::websocket::Message::binary(data)).await.is_err() {
                break;
            };
        }
        drop(moov);
        // Start with IFrame
        while let Ok(msg) = receiver.recv().await {
            if msg.i_frame {
                if sink.send(poem::web::websocket::Message::binary(msg.data.clone())).await.is_err() {
                    return;
                }
                break;
            }
        }
        while let Ok(msg) = receiver.recv().await {
            if sink.send(poem::web::websocket::Message::binary(msg.data.clone())).await.is_err() {
                break;
            }
        }
    })
}

pub fn get_moov_header(recv: &Receiver<Buffer>) -> Arc<Vec<Vec<u8>>> {
    let mut moov: Vec<Vec<u8>> = Vec::new();
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
    Arc::new(moov)
}

pub struct ParsedBuffer {
    data: Vec<u8>,
    i_frame: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GStreamer
    gstreamer::init()?;

    let (send, recv) = std::sync::mpsc::channel();
    let pipeline = video::build_gstreamer_pipline(send)?;


    let (tx, _) = tokio::sync::broadcast::channel::<Arc<ParsedBuffer>>(1024);
    let tx2 = tx.clone();

    
    
    // Start playing
    pipeline.set_state(State::Playing)?;
    println!("Pipline started");
    
    let moov = get_moov_header(&recv);

    std::thread::spawn(move || {
        let mut pts = None;
        let mut vec = Vec::with_capacity(1024);


        println!("Started recv");
        let mut key_frame = true;
        while let Ok(buffer) = recv.recv() {
            

            println!("pts: {:?}", pts);
            pts = buffer.dts();

            let mapa = buffer.map_readable().unwrap();
            let mut slice = mapa.to_vec();

            // Check for I FRAME
            if buffer.flags().iter().any(|f| {
                BufferFlags::DELTA_UNIT == f
            }) {
                key_frame = false;
            };

            // moof 6d6f 6f66
            if slice[4] == 0x6d && slice[5] == 0x6f && slice[6] == 0x6f && slice[7] == 0x66 {
                if let Err(_) = tx.send(Arc::new(ParsedBuffer {
                    data: vec.clone(),
                    i_frame: key_frame
                })) {
                    // TODO log and handle error
                }
                vec.clear();
                key_frame = true;
            }

            vec.append(&mut slice);

        }
    });

    let moov2 = Arc::clone(&moov);
    let file_sink_subscirber = tx2.subscribe();
    tokio::spawn(async move {
        file_sink::file_saver(file_sink_subscirber, moov2).await;
    });



    let app = Route::new()
        .at("/ws",
            get(ws)
            .data(tx2)
            .data(moov)
        );

    let _ = Server::new(TcpListener::bind("0.0.0.0:3000"))
        .run(app)
        .await;

    // Stop the pipeline
    pipeline.set_state(State::Null)?;

    

    Ok(())
}