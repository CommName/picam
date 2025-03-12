use std::collections::HashMap;

use v4l::{framesize::FrameSizeEnum, video::Capture};



#[derive(Debug)]
pub struct Device {
    pub path: String,
    pub capabilities: Vec<Capabilties>
}


#[derive(Debug)]
pub struct Capabilties {
    pub format: String,
    pub resolution: Vec<FrameSizeEnum>
}



impl  Device {
    
    pub fn devices() -> HashMap<String, Self> {

        let mut ret = HashMap::new();

        for dev in v4l::context::enum_devices() {
            let path = dev.path();

            if let Ok(device) = v4l::Device::with_path(path) {
                let mut caps = Vec::new();
                for fmt in device.enum_formats().unwrap_or_default() {
                    if let Ok(resolution) = device.enum_framesizes(fmt.fourcc) {
                        caps.push(Capabilties {
                            format: fmt.fourcc.str().map(|s| s.to_string()).unwrap(),
                            resolution: resolution.into_iter().map(|f| f.size).collect()
                        })
                    }

                }
                if !caps.is_empty() {
                    let path  = path.to_str().map(|s| s.to_string()).unwrap_or_default();
                    ret.insert( path.clone(), Device {
                        path,
                        capabilities: caps
                    });
                }
            }
        }

        ret
    }
}