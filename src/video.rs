
use std::sync::Arc;

use tokio::sync::broadcast::Sender;

use gstreamer::{prelude::*, BufferFlags, FlowSuccess};
use gstreamer::{ElementFactory, Pipeline};
use gstreamer_app::AppSink;

use crate::config::Config;
use crate::ParsedBuffer;

/*
gst-launch-1.0 -e v4l2src \
    ! video/x-raw, format=I420 ! x264enc key-int-max=10 ! mp4mux ! autovideosink

*/

pub fn build_gstreamer_pipline(send: Sender<Arc<ParsedBuffer>>, config: Config) -> Result<Pipeline, String> {

    // Create the elements
    let pipeline = Pipeline::new();

    let v4l2src = ElementFactory::make("v4l2src", )
        .name("v4l2src")
        .property("device", config.source)
        .property("num-buffers", -1)
        .build()
        .unwrap();

    let videoconvert = ElementFactory::make_with_name("videoconvert", Some("videoconvert")).unwrap();
    let capsfilter: gstreamer::Element = ElementFactory::make("capsfilter")
        .name("capsfilter")
        .property("caps", gstreamer::Caps::builder("video/x-raw")
            .field("format", "I420")
            .field("framerate",  gstreamer::Fraction::new(30, 1))
            .build()
        )
        .build()
        .unwrap();

    let timeoverlay = ElementFactory::make_with_name("timeoverlay", Some("timeoverlay")).unwrap();

    let x264enc = ElementFactory::make("x264enc")
        .name("x264enc")
        .property("key-int-max", 60u32)
        .property("b-adapt", false)
        .property("b-pyramid", false)
        .property("bframes", 0u32)
        .property_from_str("speed-preset", "ultrafast")
        .property_from_str("tune", "zerolatency")
        .build()
        .unwrap();

    let h264parse = ElementFactory::make_with_name("h264parse", Some("h264parse")).unwrap();

    let mpegtsmux = ElementFactory::make("mp4mux")
        .name("mp4mux")
        .property("streamable", true)
        .property("force-chunks", true)
        .property("fragment-duration", 1u32)
        .property("faststart", true)
        .build()
        .unwrap();

    let appsink = AppSink::builder()
        .name("app_sink")
        .build();


    // Add elements to the pipeline
    pipeline.add_many(&[
        &v4l2src,
        &timeoverlay,
        &videoconvert,
        &capsfilter,
        &x264enc,
        &h264parse,
        &mpegtsmux,
        appsink.upcast_ref(),
    ])
    .unwrap();

    // Link elements in the pipeline
    gstreamer::Element::link_many(&[&v4l2src, &videoconvert, &capsfilter, &timeoverlay, &x264enc, &h264parse, &mpegtsmux, &appsink.upcast_ref()]).unwrap();


    let mut vec = Vec::with_capacity(1024);


    println!("Started recv");
    let mut key_frame = true;
    let mut timestamp = None;
    let mut number_of_messages_to_forward = 0;

    appsink.set_callbacks(gstreamer_app::AppSinkCallbacks::builder()
        .new_sample(move |app_sink| {
            if let Ok(sample) = app_sink.pull_sample() {
                if let Some(buffer) = sample.buffer_owned() {
     
                    if let Some(pts) = buffer.pts() {
                        let _ = timestamp.insert(pts);
                    }
            
                    let mapa = buffer.map_readable().unwrap();
                    let mut slice = mapa.to_vec();

                    if number_of_messages_to_forward < 2 {
                        let _ =send.send(Arc::new(ParsedBuffer{
                            data: slice,
                            key_frame: true,
                            timestamp: None,
                        }));
                        number_of_messages_to_forward += 1; 
                        return Ok(FlowSuccess::Ok)
                    }
            
                        // Check for I FRAME
                    if buffer.flags().iter().any(|f| {
                        BufferFlags::DELTA_UNIT == f
                    }) {
                        key_frame = false;
                    };
            
                    // moof 6d6f 6f66
                    if slice[4] == 0x6d && slice[5] == 0x6f && slice[6] == 0x6f && slice[7] == 0x66 {
                        if let Err(_) = send.send(Arc::new(ParsedBuffer {
                            data: vec.clone(),
                            key_frame,
                            timestamp
                        })) {
                                // TODO log and handle error
                            }
                            vec.clear();
                            key_frame = true;
                            timestamp.take();
                        }
                        vec.append(&mut slice);
            
                }
            }
            
            Ok(FlowSuccess::Ok)
        }).build()
    );

    Ok(pipeline)
}
