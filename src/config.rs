use std::collections::HashMap;

use v4l::framesize::FrameSizeEnum;

use crate::sys::Device;



#[derive(Debug, Clone)]
pub struct Config {
    pub source: String,
    pub use_cam_builtin_encoder: bool,
    pub width: i32,
    pub height: i32
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

    pub fn find_optimal_settings(devices: HashMap<String, Device>) -> Self {

        let mut source = "".to_string();
        let mut use_cam_builtin_encoder = false;
        let mut max_width = 0;
        let mut max_height = 0;
        let mut max_resolition = 0;

        for (_, device) in devices {
            for cap in device.capabilities {
                let support_for_builtin_encoder = format_supports_builtin_encoder(&cap.format);
                if support_for_builtin_encoder || !use_cam_builtin_encoder {
                    for res in cap.resolution {
                        let (res, width, height) = match res {
                            FrameSizeEnum::Discrete(d) => {
                                (d.width *d.height ,d.width, d.height)
                            },
                            FrameSizeEnum::Stepwise(s) => {
                                (s.max_width * s.max_height, s.max_width, s.max_height)
                            }
                        };

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
