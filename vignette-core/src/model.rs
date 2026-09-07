use std::{collections::HashMap, sync::OnceLock};

use crate::hid::HidDevInfo;

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

enum ProtoName {
    Weisheng,
}

fn protocol_map() -> &'static HashMap<DevSpec, ProtoName> {
    static PROTOCOL_MAP: OnceLock<HashMap<DevSpec, ProtoName>> = OnceLock::new();
    PROTOCOL_MAP.get_or_init(|| {
        HashMap::from([
            (
                DevSpec {
                    vendor_id: 0x320F,
                    product_id: 0x5055,
                    usage_page: 0xFF1C,
                    usage_id: 0x92,
                },
                ProtoName::Weisheng,
            ),
            (
                DevSpec {
                    vendor_id: 0x320F,
                    product_id: 0x5088,
                    usage_page: 0xFF1C,
                    usage_id: 0x92,
                },
                ProtoName::Weisheng,
            ),
        ])
    })
}

pub fn is_supported(dev: &HidDevInfo) -> bool {
    protocol_map().contains_key(&dev.into())
}
