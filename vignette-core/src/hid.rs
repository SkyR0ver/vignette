use futures::StreamExt;

use crate::error::{HidError, HidResult};

type HidDevice = async_hid::Device;
pub type HidDevInfo = async_hid::DeviceInfo;
pub type HidDevReaderWriter = async_hid::DeviceReaderWriter;

pub(crate) async fn get_all() -> HidResult<Vec<HidDevInfo>> {
    let out = async_hid::HidBackend::default()
        .enumerate()
        .await?
        .map(HidDevice::to_device_info)
        .collect::<Vec<_>>()
        .await;
    Ok(out)
}

pub(crate) async fn open(devinfo: &HidDevInfo) -> HidResult<HidDevReaderWriter> {
    let device = async_hid::HidBackend::default()
        .query_devices(&devinfo.id)
        .await?
        .next()
        .ok_or(HidError::NotConnected)?;
    Ok(device.open().await?)
}
