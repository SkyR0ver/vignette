use async_hid::{AsyncHidRead, AsyncHidWrite};
use futures::StreamExt;

use crate::error::{HidError, HidResult};

type HidDevice = async_hid::Device;
pub type HidDevInfo = async_hid::DeviceInfo;

pub struct HidDevReader(async_hid::DeviceReader);

impl HidDevReader {
    pub async fn read_input_report(&mut self) -> HidResult<(u8, Vec<u8>)> {
        let mut buf = vec![0u8; 64];
        let len = self.0.read_input_report(&mut buf).await?;

        let report_id = buf[0];
        let data = buf[..len].to_vec();
        Ok((report_id, data))
    }
}

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
    Ok((HidDevReader(reader), HidDevWriter(writer)))
}
