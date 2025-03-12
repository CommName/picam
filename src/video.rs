
use std::collections::HashMap;
use std::sync::Arc;

use gstreamer::Element;
use tokio::sync::broadcast::{Receiver, Sender};
use log::*;

use gstreamer::{prelude::*, BufferFlags, FlowSuccess};
use gstreamer::{ElementFactory, Pipeline};
use gstreamer_app::AppSink;
use tokio::sync::RwLock;
use v4l::framesize::FrameSizeEnum;


use crate::models::PipelineConfig;
use crate::sys::Device;
use crate::ParsedBuffer;


#[derive(Clone, Debug)]
pub struct Config {
    source: String,
    use_cam_builtin_encoder: bool,
    width: i32,
    height: i32
}


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


impl Config {
    pub fn from_env() -> Self {
        let source = std::env::var("SOURCE")
            .unwrap_or_else(|_| String::from("/dev/video0"));

        let use_cam_builtin_encoder = std::env::var("BUILT_IN_ENCODER")
            .map(|p| p.parse::<bool>().ok())
            .ok()
            .flatten()
            .unwrap_or(true);

        let width = std::env::var("WIDTH")
            .map(|p| p.parse::<i32>().ok())
            .ok()
            .flatten()
            .unwrap_or(1280);

        let height = std::env::var("HEIGHT")
            .map(|p| p.parse::<i32>().ok())
            .ok()
            .flatten()
            .unwrap_or(720);
        Self {
            source,
            use_cam_builtin_encoder,
            width,
            height
        }
    }

    pub fn find_optimal_settings(devices: &HashMap<String, Device>, config: PipelineConfig) -> Self {

        let mut source = "".to_string();
        let mut use_cam_builtin_encoder = false;
        let mut max_width = 0;
        let mut max_height = 0;
        let mut max_resolition = 0;


        for (_, device) in devices {
            if check_set_parameter(&device.path.to_string(), &config.source) {
                continue;
            }

            for cap in device.capabilities.iter() {
                let support_for_builtin_encoder = format_supports_builtin_encoder(&cap.format);
                if check_set_parameter(&support_for_builtin_encoder, &config.use_cam_builtin_encoder) {
                    continue;
                }
                if support_for_builtin_encoder || !use_cam_builtin_encoder {
                    for res in cap.resolution.iter() {
                        let (res, width, height) = match res {
                            FrameSizeEnum::Discrete(d) => {
                                (d.width *d.height ,d.width, d.height)
                            },
                            FrameSizeEnum::Stepwise(s) => {
                                let width = config.width.clone().unwrap_or(0).max(s.max_width);
                                let height = config.height.clone().unwrap_or(0).max(s.max_height);
                                // TODO keep the aspect ratio
                                (width * height, width, height)
                            }
                        };
                        if check_set_parameter(&width, &config.width) ||
                            check_set_parameter(&height, &config.height) {
                            continue;
                        }

                        if max_resolition < res {
                            source = device.path.clone();
                            use_cam_builtin_encoder = support_for_builtin_encoder;
                            max_height = height;
                            max_width = width;
                            max_resolition = res; 
                        }
                    }
                }
            }
        }

        Self {
            source,
            use_cam_builtin_encoder,
            width: max_width as i32,
            height: max_height as i32
        }
    }
}


fn format_supports_builtin_encoder(format: &str) -> bool {
    format.to_uppercase() == "H264"
}

fn check_set_parameter<T: PartialEq>(parm: &T, set_parm: &Option<T>) -> bool {
    if let Some(ref set_parm) = set_parm {
        return parm != set_parm;
    }
    return false;
}