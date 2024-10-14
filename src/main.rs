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
/*
gst-launch-1.0 -e v4l2src \
    ! videoconvert ! video/x-raw, format=I420 ! x264enc key-int-max=10 ! queue  ! tee name=t ! \
    queue ! h264parse ! splitmuxsink location=video%02d.mkv max-size-time=10000000000 muxer-factory=matroskamux muxer-properties="properties,streamable=true" \
    t. ! queue ! mpegtsmux ! udpsink host=224.0.0.4 port=5000

*/


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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GStreamer
    gstreamer::init()?;

    // Create the elements
    let pipeline = Pipeline::new();

    let v4l2src = ElementFactory::make("v4l2src", )
        .name("v4l2src")
        //.property("device", "/dev/video1")
        //.property("num-buffers", 500)
        .build()?;

    let videoconvert = ElementFactory::make_with_name("videoconvert", Some("videoconvert")).unwrap();
    let capsfilter = ElementFactory::make("capsfilter")
        .name("capsfilter")
        .property("caps", gstreamer::Caps::builder("video/x-raw")
            .field("format", "I420")
            .build()
        )
        .build()?;

    let queue1 = ElementFactory::make_with_name("queue", Some("queue1")).unwrap();
    let x264enc = ElementFactory::make("x264enc")
        .name("x264enc")
        .property("key-int-max", 1u32)
        .property("b-adapt", false)
        .property("b-pyramid", false)
        .property("bframes", 0u32)
        .property_from_str("speed-preset", "ultrafast")
        //.property_from_str("tune", "zerolatency")
        .build()?;

    let h264parse = ElementFactory::make_with_name("h264parse", Some("h264parse"))?;

    let mpegtsmux = ElementFactory::make("mp4mux")
        .name("mp4mux")
        .property("streamable", true)
        .property("force-chunks", true)
        .property("fragment-duration", 1u32)
        //.property_from_str("fragment-mode", "first-moov-then-finalise")
        .property("faststart", true)
        .build()?;

    let appsink = AppSink::builder()
        .name("app_sink")
        .build();


    // Add elements to the pipeline
    pipeline.add_many(&[
        &v4l2src, &videoconvert, &capsfilter, &queue1, &x264enc, &h264parse, &mpegtsmux, appsink.upcast_ref(),
    ])?;

    // Link elements in the pipeline
    gstreamer::Element::link_many(&[&v4l2src, &videoconvert, &capsfilter, &queue1, &x264enc, &h264parse, &mpegtsmux, &appsink.upcast_ref()])?;

    let (send, recv) = std::sync::mpsc::channel();

    let mut framecount = 0;

    appsink.set_callbacks(gstreamer_app::AppSinkCallbacks::builder()
        .new_sample(move |app_sink| {
            if let Ok(sample) = app_sink.pull_sample() {
                if let Some(buffer) = sample.buffer_owned() {
                    send.send((buffer, framecount));
                    framecount = (framecount + 1) % 2;
                    // let dts = buffer.dts();
                    // let pts = buffer.pts();
                    // let size = buffer.size();

                    // let mapa = buffer.map_readable().unwrap();
                    // let slice = mapa.to_vec();
                    // let size: usize = slice.len();
                    // println!("DTS: {dts:?} PTS: {pts:?} SIZE: {size}");
                }
            }
            
            Ok(FlowSuccess::Ok)
        }).build()
    );

    let (tx, _) = tokio::sync::broadcast::channel::<Vec<u8>>(1024);
    let tx2 = tx.clone();
    let moov: Arc<RwLock<Vec<Vec<u8>>>> = Arc::new(RwLock::new(Vec::new()));
    let moov2 = Arc::clone(&moov);


    std::thread::spawn(move || {
        let mut pts = None;
        let mut vec = Vec::with_capacity(1024);
        let mut number_of_buffs = 0;
        {
            let mut moov = moov.write().unwrap();
            while number_of_buffs < 2 {
                if let Ok((buffer, framecount)) = recv.recv() {
                    let mapa = buffer.map_readable().unwrap();
                    let slice = mapa.to_vec();

                    moov.push(slice);
                    number_of_buffs += 1;
                } else {
                    break;
                }
            }
        }

        while let Ok((buffer, framecount)) = recv.recv() {
            //println!("Buffer {:?}", buffer);
            if false && pts == buffer.dts() {
                if vec.len() > 0 {
                }
                tx.send(vec.clone());
                println!("pts: {:?}", pts);
                pts = buffer.dts();
                // TODO send vector
                vec.clear();
            }
            let mapa = buffer.map_readable().unwrap();
            let mut slice = mapa.to_vec();
            tx.send(slice);
            //vec.append(&mut slice);
        }
    });


    // Start playing
    pipeline.set_state(State::Playing)?;

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