mod error;
mod hid;
mod model;
mod protocol;
mod util;

use crate::error::HidResult;
use crate::hid::{HidDevInfo, HidDevReaderWriter};

pub use protocol::weisheng::{
    get_battery_level, get_firmware_version, get_function_info, reset, set_function_info,
};

pub async fn get_all() -> HidResult<Vec<HidDevInfo>> {
    hid::get_all().await
}

pub async fn get() -> HidResult<Vec<HidDevInfo>> {
    let supported_devices = get_all()
        .await?
        .into_iter()
        .filter(model::is_supported)
        .collect();
    Ok(supported_devices)
}

pub async fn open(dev: &HidDevInfo) -> HidResult<HidDevReaderWriter> {
    hid::open(dev).await
}
