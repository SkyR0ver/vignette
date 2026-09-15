mod error;
mod hid;
mod model;
mod protocol;
mod util;

use crate::error::{HidResult, ProtoResult};
use crate::hid::HidDevInfo;
use crate::model::Keyboard;

pub async fn get_all() -> HidResult<Vec<HidDevInfo>> {
    hid::get_all().await
}

pub async fn get() -> HidResult<Vec<HidDevInfo>> {
    let supported_devices = get_all()
        .await?
        .into_iter()
        .filter(protocol::is_supported)
        .collect();
    Ok(supported_devices)
}

pub async fn open(info: &HidDevInfo) -> ProtoResult<Keyboard> {
    let dev = hid::find_device(info).await?;
    Ok(Keyboard::new(dev, info.clone())?)
}
