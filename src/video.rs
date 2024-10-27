
use std::sync::mpsc::Sender;

use gstreamer::{prelude::*, Buffer, FlowSuccess};
use gstreamer::{ElementFactory, Pipeline};
use gstreamer_app::AppSink;

/*
gst-launch-1.0 -e v4l2src \
    ! video/x-raw, format=I420 ! x264enc key-int-max=10 ! mp4mux ! autovideosink

*/

pub fn build_gstreamer_pipline(send: Sender<Buffer>) -> Result<Pipeline, glib::BoolError> {

    // Create the elements
    let pipeline = Pipeline::new();

    let v4l2src = ElementFactory::make("v4l2src", )
        .name("v4l2src")
        //.property("device", "/dev/video1")
        .property("num-buffers", -1)
        .build()?;

    let videoconvert = ElementFactory::make_with_name("videoconvert", Some("videoconvert")).unwrap();
    let capsfilter: gstreamer::Element = ElementFactory::make("capsfilter")
        .name("capsfilter")
        .property("caps", gstreamer::Caps::builder("video/x-raw")
            .field("format", "I420")
            .field("framerate",  gstreamer::Fraction::new(5, 1))
            .build()
        )
        .build()?;

    let timeoverlay = ElementFactory::make_with_name("timeoverlay", Some("timeoverlay")).unwrap();

    let x264enc = ElementFactory::make("x264enc")
        .name("x264enc")
        .property("key-int-max", 60u32)
        .property("b-adapt", false)
        .property("b-pyramid", false)
        .property("bframes", 0u32)
        .property_from_str("speed-preset", "ultrafast")
        .property_from_str("tune", "zerolatency")
        .build()?;

    let h264parse = ElementFactory::make_with_name("h264parse", Some("h264parse"))?;

    let mpegtsmux = ElementFactory::make("mp4mux")
        .name("mp4mux")
        .property("streamable", true)
        .property("force-chunks", true)
        .property("fragment-duration", 1u32)
        .property("faststart", true)
        .build()?;

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
    ])?;

    // Link elements in the pipeline
    gstreamer::Element::link_many(&[&v4l2src, &videoconvert, &capsfilter, &timeoverlay, &x264enc, &h264parse, &mpegtsmux, &appsink.upcast_ref()])?;

    appsink.set_callbacks(gstreamer_app::AppSinkCallbacks::builder()
        .new_sample(move |app_sink| {
            if let Ok(sample) = app_sink.pull_sample() {
                if let Some(buffer) = sample.buffer_owned() {
                    if let Err(_) = send.send(buffer) {
                        // TODO log and handle error
                    }
                }
            }
            
            Ok(FlowSuccess::Ok)
        }).build()
    );

    Ok(pipeline)
}
