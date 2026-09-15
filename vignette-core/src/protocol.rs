mod weisheng;

use std::{collections::HashMap, sync::OnceLock};

use crate::{
    error::ProtoResult,
    hid::{HidDevInfo, HidDevReaderWriter},
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct DevSpec {
    vendor_id: u16,
    product_id: u16,
    usage_page: u16,
    usage_id: u16,
}

impl From<&HidDevInfo> for DevSpec {
    fn from(dev: &HidDevInfo) -> Self {
        DevSpec {
            vendor_id: dev.vendor_id,
            product_id: dev.product_id,
            usage_page: dev.usage_page,
            usage_id: dev.usage_id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardProtocol {
    Weisheng,
}

impl KeyboardProtocol {
    pub async fn get_battery_level(&self, rw: &mut HidDevReaderWriter) -> ProtoResult<u8> {
        match self {
            KeyboardProtocol::Weisheng => weisheng::get_battery_level(rw).await,
        }
    }

    pub async fn get_charging_status(&self, rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
        match self {
            KeyboardProtocol::Weisheng => weisheng::get_charging_status(rw).await,
        }
    }

    pub async fn get_firmware_version(&self, rw: &mut HidDevReaderWriter) -> ProtoResult<u16> {
        match self {
            KeyboardProtocol::Weisheng => weisheng::get_firmware_version(rw).await,
        }
    }

    pub async fn reset(&self, rw: &mut HidDevReaderWriter) -> ProtoResult<()> {
        match self {
            KeyboardProtocol::Weisheng => weisheng::reset(rw).await,
        }
    }
}

fn protocol_map() -> &'static HashMap<DevSpec, KeyboardProtocol> {
    static PROTOCOL_MAP: OnceLock<HashMap<DevSpec, KeyboardProtocol>> = OnceLock::new();
    PROTOCOL_MAP.get_or_init(|| {
        HashMap::from([
            (
                DevSpec {
                    vendor_id: 0x320F,
                    product_id: 0x5055,
                    usage_page: 0xFF1C,
                    usage_id: 0x92,
                },
                KeyboardProtocol::Weisheng,
            ),
            (
                DevSpec {
                    vendor_id: 0x320F,
                    product_id: 0x5088,
                    usage_page: 0xFF1C,
                    usage_id: 0x92,
                },
                KeyboardProtocol::Weisheng,
            ),
        ])
    })
}

pub fn match_protocol(dev: &HidDevInfo) -> Option<KeyboardProtocol> {
    protocol_map().get(&dev.into()).copied()
}

pub fn is_supported(dev: &HidDevInfo) -> bool {
    protocol_map().contains_key(&dev.into())
}
