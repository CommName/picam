

use std::{collections::HashMap, hash::Hash};

use crate::sys::Device;

use super::DeviceStorage;

pub struct MemoryDeviceStorage {
    devices: HashMap<String, Device>
}

impl Default for MemoryDeviceStorage {
    fn default() -> Self {
        let devices = Device::devices();
        Self {
            devices
        }
    }
}

#[async_trait::async_trait]
impl DeviceStorage for MemoryDeviceStorage {
    async fn devices(&self) ->  &HashMap<String, Device> {
        &self.devices
    }
}