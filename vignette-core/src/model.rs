use std::{collections::HashMap, sync::OnceLock};

use crate::{
    error::{HidResult, ProtoError, ProtoResult},
    hid::{HidDevInfo, HidDevReaderWriter, HidDevice},
    protocol::weisheng,
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

pub struct Keyboard {
    dev: HidDevice,
    protocol: ProtoName,
}

impl Keyboard {
    pub fn new(dev: HidDevice, info: HidDevInfo) -> ProtoResult<Self> {
        let protocol = *protocol_map()
            .get(&DevSpec::from(&info))
            .ok_or_else(|| ProtoError::NotSupported(info.into()))?;
        Ok(Self { dev, protocol })
    }

    async fn open(&self) -> HidResult<HidDevReaderWriter> {
        self.dev.open().await
    }

    pub async fn get_battery_level(&self) -> ProtoResult<u8> {
        let mut rw = self.open().await?;
        match self.protocol {
            ProtoName::Weisheng => weisheng::get_battery_level(&mut rw)
                .await
                .map(|(level, _charging)| level),
        }
    }

    pub async fn get_charging_status(&self) -> ProtoResult<bool> {
        let mut rw = self.open().await?;
        match self.protocol {
            ProtoName::Weisheng => weisheng::get_battery_level(&mut rw)
                .await
                .map(|(_level, charging)| charging),
        }
    }

    pub async fn get_firmware_version(&self) -> ProtoResult<u16> {
        let mut rw = self.open().await?;
        match self.protocol {
            ProtoName::Weisheng => weisheng::get_firmware_version(&mut rw).await,
        }
    }

    pub async fn reset(&self) -> ProtoResult<()> {
        let mut rw = self.open().await?;
        match self.protocol {
            ProtoName::Weisheng => weisheng::reset(&mut rw).await,
        }
    }
}
