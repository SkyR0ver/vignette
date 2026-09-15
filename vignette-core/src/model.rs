use crate::{
    error::{HidResult, ProtoError, ProtoResult},
    hid::{HidDevInfo, HidDevReaderWriter, HidDevice},
    protocol::{KeyboardProtocol, match_protocol},
};

pub struct Keyboard {
    dev: HidDevice,
    protocol: KeyboardProtocol,
}

impl Keyboard {
    pub fn new(dev: HidDevice, info: HidDevInfo) -> ProtoResult<Self> {
        let protocol =
            match_protocol(&info).ok_or_else(|| ProtoError::NotSupported(info.into()))?;
        Ok(Self { dev, protocol })
    }

    async fn open(&self) -> HidResult<HidDevReaderWriter> {
        self.dev.open().await
    }

    pub async fn get_battery_level(&self) -> ProtoResult<u8> {
        self.protocol
            .get_battery_level(&mut self.open().await?)
            .await
    }

    pub async fn get_charging_status(&self) -> ProtoResult<bool> {
        self.protocol
            .get_charging_status(&mut self.open().await?)
            .await
    }

    pub async fn get_firmware_version(&self) -> ProtoResult<u16> {
        self.protocol
            .get_firmware_version(&mut self.open().await?)
            .await
    }

    pub async fn reset(&self) -> ProtoResult<()> {
        self.protocol.reset(&mut self.open().await?).await
    }
}
