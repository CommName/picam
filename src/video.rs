
use std::sync::Arc;

use gstreamer::{Element, MessageType};
use tokio::sync::broadcast::{Receiver, Sender};
use log::*;

use gstreamer::{prelude::*, BufferFlags, FlowSuccess};
use gstreamer::{ElementFactory, Pipeline};
use gstreamer_app::AppSink;
use tokio::sync::RwLock;

use crate::config::Config;
use crate::ParsedBuffer;

/*
gst-launch-1.0 -e v4l2src \
    ! video/x-raw, format=I420 ! x264enc key-int-max=10 ! mp4mux ! autovideosink

*/
/*
gst-launch-1.0 v4l2src ! video/x-h264, width=1280, height=720 ! h264parse ! mp4mux streamable=true force-chunks=true faststart=true ! fakesink
*/

fn short_pipeline(config: &Config) -> Vec<Element> { 
    let capsfilter: gstreamer::Element = ElementFactory::make("capsfilter")
        .name("capsfilter")
        .property("caps", gstreamer::Caps::builder("video/x-h264")
            .field("format", "I420")
            .field("width", config.width)
            .field("height", config.height)
            .field("framerate",  gstreamer::Fraction::new(30, 1))
            .build()
        )
        .build()
        .unwrap();

    vec![capsfilter]
}

fn long_pipeline(config: &Config) -> Vec<Element> {
    println!("Using software encoder");
    let videoconvert = ElementFactory::make_with_name("videoconvert", Some("videoconvert")).unwrap();
    let capsfilter: gstreamer::Element = ElementFactory::make("capsfilter")
        .name("capsfilter")
        .property("caps", gstreamer::Caps::builder("video/x-raw")
            .field("format", "I420")
            .field("width", config.width)
            .field("height", config.height)
            .field("framerate",  gstreamer::Fraction::new(30, 1))
            .build()
        )
        .build()
        .unwrap();

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

    vec![videoconvert, capsfilter, x264enc]
}

pub fn build_gstreamer_pipline(send: Sender<Arc<ParsedBuffer>>, config: &Config) -> Result<Pipeline, String> {
    debug!("Createing new pipeline");
    // Create the elements
    let pipeline = Pipeline::new();

    let v4l2src = ElementFactory::make("v4l2src", )
        .name("v4l2src")
        .property("device", config.source.clone())
        .property("num-buffers", -1)
        .build()
        .unwrap();

    let video_elements = if config.use_cam_builtin_encoder {
        short_pipeline(&config)
    } else {
        long_pipeline(&config) 
    };


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
    pipeline.add_many(&video_elements)
        .unwrap();

    pipeline.add_many(&[
        &v4l2src,
        &h264parse,
        &mpegtsmux,
        appsink.upcast_ref(),
    ])
    .unwrap();

    // Link elements in the pipeline
    gstreamer::Element::link_many(&[
        &v4l2src,
        video_elements.first().unwrap()
    ]).unwrap();
    if video_elements.len() > 1 {
        gstreamer::Element::link_many(&video_elements).unwrap()
    }
    gstreamer::Element::link_many(&[
        video_elements.last().unwrap(),
        &h264parse,
        &mpegtsmux,
        &appsink.upcast_ref()
    ]).unwrap();


    let mut vec = Vec::with_capacity(1024);

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
                            message_type: if number_of_messages_to_forward == 0 {
                                crate::MessageType::FirstFrame
                            } else {
                                crate::MessageType::MoovPacket
                            },
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
                            message_type: if key_frame {
                                crate::MessageType::KeyFrame
                            } else {
                                crate::MessageType::Fragment
                            },
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


pub async  fn init_moov_header(mut recv: Receiver<Arc<ParsedBuffer>>, moov: Arc<RwLock<Vec<Vec<u8>>>>) -> Arc<Vec<Vec<u8>>> {

    loop { 
        let mut moov = moov.write().await;
        loop {
            if let Ok(buffer) = recv.recv().await {
                if crate::MessageType::FirstFrame != buffer.message_type
                    && crate::MessageType::MoovPacket != buffer.message_type {
                        break;
                }
                moov.push(buffer.data.clone());
            } else {
                break;
            }
        }

        drop(moov);

        while let Ok(buffer) = recv.recv().await {
            if let crate::MessageType::FirstFrame = buffer.message_type {
                break;
            }
        }
    }
}