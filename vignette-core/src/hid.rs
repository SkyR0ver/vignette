use async_hid::AsyncHidWrite;
use futures::StreamExt;

use crate::error::{HidError, HidResult};

type HidDevice = async_hid::Device;
pub type HidDevInfo = async_hid::DeviceInfo;

pub struct HidDevWriter(async_hid::DeviceWriter);

impl HidDevWriter {
    pub async fn write_output_report(&mut self, report_id: u8, data: &[u8]) -> HidResult<()> {
        let mut buf = Vec::with_capacity(data.len() + 1);
        buf.push(report_id);
        buf.extend_from_slice(data);
        self.0.write_output_report(&buf).await?;
        Ok(())
    }
}

pub type HidDevReader = async_hid::DeviceReader;
pub type HidDevReaderWriter = (HidDevReader, HidDevWriter);

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
    let (reader, writer) = device.open().await?;
    Ok((reader, HidDevWriter(writer)))
}
