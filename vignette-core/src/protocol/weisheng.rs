use std::{cmp::min, io::Cursor};

use binrw::{BinRead, BinWrite, binrw};

use crate::{
    error::{BinResult, HidResult, ProtoError, ProtoResult},
    hid::{HidDevReader, HidDevWriter},
};

pub const WS_REPORT_ID: u8 = 4;

/// Data frame in the weisheng protocol.
///
/// The length of the frame is 63 bytes at most. If the data to be
/// transmitted is too long, it will be fragmented into multiple frames.
#[binrw]
#[brw(magic(b"\0\0"), little)]
#[derive(Debug)]
pub struct WsFrame {
    /// Command code
    cmd: WsCmd,
    /// Length of the payload
    #[br(try_calc = cmd.payload_len().try_into(), pad_after = 1)]
    #[bw(try_calc = u8::try_from(data.len()))]
    len: u8,
    /// Offset of the payload if fragmented
    offset: u16,
    /// Payload data
    #[br(count = len)]
    #[brw(pad_before = 1)]
    data: Vec<u8>,
}

impl WsFrame {
    /// Creates a new frame with the given command and data. If the data is too
    /// long, it will be fragmented into multiple frames.
    ///
    /// Passing `None` as data will create frames with zeroed payloads, as if
    /// a slice of the expected data length for the command was given.
    pub fn new(cmd: WsCmd, data: Option<&[u8]>) -> Vec<Self> {
        let frame_len = cmd.payload_len();
        match data {
            Some(data) => data
                .chunks(frame_len)
                .enumerate()
                .map(|(i, chunk)| WsFrame {
                    cmd: cmd,
                    offset: (i * frame_len) as u16,
                    data: chunk.to_vec(),
                })
                .collect(),
            None => (0..cmd.data_len().div_ceil(frame_len))
                .map(|i| WsFrame {
                    cmd: cmd,
                    offset: (i * frame_len) as u16,
                    data: vec![0u8; frame_len],
                })
                .collect(),
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> BinResult<Self> {
        let mut cursor = Cursor::new(bytes);
        WsFrame::read(&mut cursor)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::with_capacity(64));
        self.write(&mut buf)
            .expect("Failed to write WsFrame to bytes");
        buf.into_inner()
    }
}

async fn send(writer: &mut HidDevWriter, request: &WsFrame) -> HidResult<()> {
    let req_data = request.to_bytes();
    writer.write_output_report(WS_REPORT_ID, &req_data).await
}

async fn recv(reader: &mut HidDevReader) -> ProtoResult<WsFrame> {
    let (report_id, resp_data) = reader.read_input_report().await?;
    if report_id != WS_REPORT_ID {
        return Err(ProtoError::InvalidReportId(report_id));
    }
    let response = WsFrame::from_bytes(&resp_data)?;
    Ok(response)
}

/// Command code in the weisheng protocol.
#[binrw]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum WsCmd {
    #[brw(magic(1u8))]
    StartQuickComm,
    #[brw(magic(2u8))]
    EndQuickComm,
    #[brw(magic(3u8))]
    GetVersion,
    #[brw(magic(5u8))]
    GetFunctionInfo,
    #[brw(magic(6u8))]
    SetFunctionInfo,
    #[brw(magic(8u8))]
    GetKey,
    #[brw(magic(9u8))]
    SetKey,
    #[brw(magic(10u8))]
    GetCustomLight,
    #[brw(magic(11u8))]
    SetCustomLight,
    #[brw(magic(13u8))]
    ResetSettings,
    #[brw(magic(21u8))]
    WriteMacro,
    #[brw(magic(26u8))]
    GetBatteryLevel,
    #[brw(magic(27u8))]
    GetLightEffectMat,
    #[brw(magic(38u8))]
    GetFnKey,
    #[brw(magic(39u8))]
    SetFnKey,
    Error,
}

impl WsCmd {
    /// Returns the expected length of the payload in frames for the command.
    fn payload_len(&self) -> usize {
        match self {
            WsCmd::GetVersion | WsCmd::GetFunctionInfo => 24,
            WsCmd::GetBatteryLevel => 2,
            _ => 56,
        }
    }

    /// Returns the expected length of complete data for the command.
    fn data_len(&self) -> usize {
        match self {
            WsCmd::GetVersion => 30,
            WsCmd::GetFunctionInfo => 35,
            WsCmd::GetBatteryLevel => 2,
            _ => 56,
        }
    }
}

async fn execute(
    reader: &mut HidDevReader,
    writer: &mut HidDevWriter,
    cmd: WsCmd,
    data: Option<&[u8]>,
) -> ProtoResult<Vec<u8>> {
    let data_len = cmd.data_len();

    let requests = WsFrame::new(cmd, data);
    let mut resp_data = vec![0u8; data_len];

    for frame in &requests {
        send(writer, frame).await?;
        let response = recv(reader).await?;
        if response.cmd != cmd {
            return Err(ProtoError::ResponseMismatch);
        }

        let start = response.offset as usize;
        let end = min(data_len, start + response.data.len());
        resp_data[start..end].copy_from_slice(&response.data[..(end - start)]);
    }

    Ok(resp_data)
}

pub async fn get_battery_level(
    reader: &mut HidDevReader,
    writer: &mut HidDevWriter,
) -> ProtoResult<(u8, bool)> {
    let battery_data = execute(reader, writer, WsCmd::GetBatteryLevel, None).await?;
    let battery_level = battery_data[0];
    let charging_status = battery_data[1] != 0;
    Ok((battery_level, charging_status))
}

pub async fn get_firmware_version(
    reader: &mut HidDevReader,
    writer: &mut HidDevWriter,
) -> ProtoResult<u16> {
    let version_data = execute(reader, writer, WsCmd::GetVersion, None).await?;
    let version_high = version_data[29] as u16;
    let version_low = version_data[28] as u16;
    Ok((version_high << 8) | version_low)
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
pub struct FunctionInfo {
    #[brw(pad_before = 1)]
    pub light_mode: LightMode,
    pub light_brightness: u8,
    pub light_speed: u8,
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub light_reverse: bool,
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub rainbow_mode: bool,
    pub static_color: [u8; 3],
    pub color_index: u8,
    pub custom_color: [u8; 3],
    pub sleep_time: u16,
    #[brw(pad_before = 3)]
    pub color_trigger_mode: u8,
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub swap_wasd: bool,
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub all_key_punchless: bool,
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub lock_win: bool,
    pub polling_rate: PollingRate,
    #[brw(pad_before = 4)]
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub mac_mode: bool,
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub enable_light: bool,
    #[brw(pad_before = 4)]
    #[br(try_map = u8::try_into)]
    #[bw(try_map = |x| u8::try_from(*x))]
    pub enable_smart_speed: bool,
    pub enable_low_latency: u8,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Debug)]
pub enum LightMode {
    Flow = 1,
    Cloud,
    Circuit,
    Judgement,
    Breath,
    Steady,
    Step,
    Ripple,
    Dash,
    Surging,
    Blossom,
    Stripes,
    Soaring,
    Swirl,
    Rain,
    Tide,
    Unity,
    Passion,
    Custom,
    Off,
}

#[binrw]
#[brw(repr = u8)]
#[derive(Debug)]
pub enum PollingRate {
    Hz1000,
    Hz500,
    Hz250,
    Hz125,
}

pub async fn get_function_info(
    reader: &mut HidDevReader,
    writer: &mut HidDevWriter,
) -> ProtoResult<FunctionInfo> {
    let info_data = execute(reader, writer, WsCmd::GetFunctionInfo, None).await?;
    let info = FunctionInfo::read(&mut Cursor::new(info_data))?;
    Ok(info)
}
