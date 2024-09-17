use std::io::Write;
use std::ops::Deref;

use byte_slice_cast::AsByteSlice;
use gstreamer::{prelude::*, FlowSuccess};
use gstreamer::{ElementFactory, Pipeline, State, StateChangeSuccess};
use gstreamer_app::AppSink;
/*
gst-launch-1.0 -e v4l2src \
    ! videoconvert ! video/x-raw, format=I420 ! x264enc key-int-max=10 ! queue  ! tee name=t ! \
    queue ! h264parse ! splitmuxsink location=video%02d.mkv max-size-time=10000000000 muxer-factory=matroskamux muxer-properties="properties,streamable=true" \
    t. ! queue ! mpegtsmux ! udpsink host=224.0.0.4 port=5000

*/
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize GStreamer
    gstreamer::init()?;

    // Create the elements
    let pipeline = Pipeline::new();

    let v4l2src = ElementFactory::make("v4l2src", )
        .name("v4l2src")
        //.property("num-buffers", 500)
        .build()?;

    let videoconvert = ElementFactory::make_with_name("videoconvert", Some("videoconvert")).unwrap();
    let queue1 = ElementFactory::make_with_name("queue", Some("queue1")).unwrap();
    let x264enc = ElementFactory::make("x264enc")
        .name("x264enc")
        .property("key-int-max", 60u32)
        .property_from_str("tune", "zerolatency")
        .build()?;

    let tee = ElementFactory::make_with_name("tee", Some("tee")).unwrap();
    let queue2 = ElementFactory::make_with_name("queue", Some("queue2")).unwrap();
    let h264parse = ElementFactory::make_with_name("h264parse", Some("h264parse"))?;
    let splitmuxsink = ElementFactory::make("splitmuxsink")
        .name("splitmuxsink")
        .property("location", "video%02d.mkv")
        .property("max-size-time", 10_000_000_000u64)
        .property_from_str("muxer-factory", "matroskamux")
        .property_from_str("muxer-properties", "properties,streamable=true")
        .build()?;


    let queue3 = ElementFactory::make_with_name("queue", Some("queue3")).unwrap();
    let mpegtsmux = ElementFactory::make_with_name("mpegtsmux", Some("mpegtsmux"))?;
    let udpsink = ElementFactory::make("udpsink")
        .name("udpsink")
        .property("host", "224.0.0.125")
        .property("port", 5000)
        .build()
        .unwrap();

    let appsink = AppSink::builder()
        .name("app_sink")
        .build();


    // Add elements to the pipeline
    pipeline.add_many(&[
        &v4l2src, &videoconvert, &queue1, &x264enc, &tee, &queue2, &h264parse,
        &splitmuxsink, &queue3, &mpegtsmux, appsink.upcast_ref(),
    ])?;

    // Link elements in the pipeline
    gstreamer::Element::link_many(&[&v4l2src, &videoconvert, &queue1, &x264enc, &tee])?;
    gstreamer::Element::link_many(&[&queue2, &h264parse, &splitmuxsink])?;
    gstreamer::Element::link_many(&[&queue3, &mpegtsmux, &appsink.upcast_ref()])?;

    // Link tee to other branches
    let tee_src_1 = tee.request_pad_simple("src_0").unwrap();
    let queue2_sink = queue2.static_pad("sink").unwrap();
    tee_src_1.link(&queue2_sink)?;

    let tee_src_2 = tee.request_pad_simple("src_1").unwrap();
    let queue3_sink = queue3.static_pad("sink").unwrap();
    tee_src_2.link(&queue3_sink)?;

    let (send, recv) = std::sync::mpsc::channel();

    appsink.set_callbacks(gstreamer_app::AppSinkCallbacks::builder()
        .new_sample(move |app_sink| {

            if let Ok(sample) = app_sink.pull_sample() {
                if let Some(buffer) = sample.buffer() {
                    let dts = buffer.dts();
                    let pts = buffer.pts();
                    let size = buffer.size();

                    let mapa = buffer.map_readable().unwrap();
                    let slice = mapa.to_vec();
                    let size = slice.len();
                    send.send(slice);
                    println!("DTS: {dts:?} PTS: {pts:?} SIZE: {size}");
                }
            }

            Ok(FlowSuccess::Ok)
        }).build()
    );

    std::thread::spawn(move || {
        let mut file = std::fs::File::create("./test.ts").unwrap();
        while let Ok(slice) = recv.recv() {
            println!("Writing to file");
            file.write(&slice).unwrap();
        }
    });

    // Start playing
    pipeline.set_state(State::Playing)?;

    // Wait until error or EOS
    let bus = pipeline.bus().unwrap();
    for msg in bus.timed_pop_filtered(gstreamer::ClockTime::NONE, &[gstreamer::MessageType::Eos, gstreamer::MessageType::Error]) {
        match msg.view() {
            gstreamer::MessageView::Error(err) => {
                eprintln!("Error received from element {:?}: {}", msg.src().map(|s| s.name()), err.error());
                eprintln!("Debugging information: {:?}", err.debug());
                break;
            }
            gstreamer::MessageView::Eos(..) => {
                println!("End of stream reached.");
                break;
            }
            _ => (),
        }
    }

    // Stop the pipeline
    pipeline.set_state(State::Null)?;

    Ok(())
}