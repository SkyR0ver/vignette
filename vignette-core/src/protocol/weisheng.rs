use std::{
    cmp::{max, min},
    io::Cursor,
};

use binrw::{BinRead, BinWrite, binrw};

use crate::{
    error::{BinResult, ProtoError, ProtoResult},
    hid::{HidDevReader, HidDevReaderWriter, HidDevWriter},
    util,
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
            None => {
                let data_len = cmd.data_len();
                let num_frames = if data_len == 0 {
                    1
                } else {
                    data_len.div_ceil(frame_len)
                };
                (0..num_frames)
                    .map(|i| WsFrame {
                        cmd: cmd,
                        offset: (i * frame_len) as u16,
                        data: vec![0u8; frame_len],
                    })
                    .collect()
            }
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> BinResult<Self> {
        let mut cursor = Cursor::new(bytes);
        WsFrame::read(&mut cursor)
    }

    pub fn to_bytes(&self) -> BinResult<Vec<u8>> {
        let mut buf = Cursor::new(Vec::with_capacity(64));
        self.write(&mut buf)?;
        Ok(buf.into_inner())
    }
}

async fn send(writer: &mut HidDevWriter, request: &WsFrame) -> ProtoResult<()> {
    let req_data = request.to_bytes()?;
    writer.write_output_report(WS_REPORT_ID, &req_data).await?;
    Ok(())
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
            WsCmd::GetVersion | WsCmd::GetFunctionInfo | WsCmd::SetFunctionInfo => 24,
            WsCmd::GetBatteryLevel => 2,
            WsCmd::ResetSettings => 0,
            _ => 56,
        }
    }

    /// Returns the expected length of complete data for the command.
    fn data_len(&self) -> usize {
        match self {
            WsCmd::GetVersion => 30,
            WsCmd::GetFunctionInfo | WsCmd::SetFunctionInfo => 35,
            WsCmd::GetBatteryLevel => 2,
            WsCmd::ResetSettings => 0,
            _ => 56,
        }
    }
}

async fn execute(
    rw: &mut HidDevReaderWriter,
    cmd: WsCmd,
    data: Option<&[u8]>,
) -> ProtoResult<Vec<u8>> {
    let (reader, writer) = rw;
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

async fn get_battery_status(rw: &mut HidDevReaderWriter) -> ProtoResult<(u8, bool)> {
    let battery_data = execute(rw, WsCmd::GetBatteryLevel, None).await?;
    let battery_level = battery_data[0];
    let charging_status = battery_data[1] != 0;
    Ok((battery_level, charging_status))
}

pub async fn get_battery_level(rw: &mut HidDevReaderWriter) -> ProtoResult<u8> {
    let (level, _) = get_battery_status(rw).await?;
    Ok(level)
}

pub async fn get_charging_status(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let (_, charging) = get_battery_status(rw).await?;
    Ok(charging)
}

pub async fn get_firmware_version(rw: &mut HidDevReaderWriter) -> ProtoResult<u16> {
    let version_data = execute(rw, WsCmd::GetVersion, None).await?;
    let version_high = version_data[29] as u16;
    let version_low = version_data[28] as u16;
    Ok((version_high << 8) | version_low)
}

pub async fn reset(rw: &mut HidDevReaderWriter) -> ProtoResult<()> {
    execute(rw, WsCmd::ResetSettings, None).await?;
    Ok(())
}

#[binrw]
#[brw(little)]
#[derive(Debug)]
pub struct FunctionInfo {
    #[brw(pad_before = 1)]
    pub light_mode: LightMode,
    pub light_brightness: u8,
    pub light_speed: u8,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
    pub light_reverse: bool,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
    pub rainbow_mode: bool,
    pub static_color: [u8; 3],
    pub color_index: u8,
    pub custom_color: [u8; 3],
    pub sleep_time: u16,
    reserved15: u8,
    reserved16: u8,
    reserved17: u8,
    pub color_trigger_mode: u8,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
    pub swap_wasd: bool,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
    pub all_key_punchless: bool,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
    pub lock_win: bool,
    pub polling_rate: PollingRate,
    reserved23: u8,
    reserved24: u8,
    reserved25: u8,
    reserved26: u8,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
    pub mac_mode: bool,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
    pub enable_light: bool,
    reserved29: u8,
    reserved30: u8,
    reserved31: u8,
    reserved32: u8,
    #[br(map = util::bool_parser)]
    #[bw(map = util::bool_writer)]
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

async fn get_function_info(rw: &mut HidDevReaderWriter) -> ProtoResult<FunctionInfo> {
    let info_data = execute(rw, WsCmd::GetFunctionInfo, None).await?;
    let info = FunctionInfo::read(&mut Cursor::new(info_data))?;
    Ok(info)
}

async fn set_function_info(rw: &mut HidDevReaderWriter, info: &FunctionInfo) -> ProtoResult<()> {
    let mut info_data = Cursor::new(Vec::with_capacity(WsCmd::SetFunctionInfo.data_len()));
    info.write(&mut info_data)?;
    let _ = execute(rw, WsCmd::SetFunctionInfo, Some(info_data.get_ref())).await?;
    Ok(())
}

pub async fn get_light_mode(rw: &mut HidDevReaderWriter) -> ProtoResult<LightMode> {
    let info = get_function_info(rw).await?;
    Ok(info.light_mode)
}

pub async fn set_light_mode(rw: &mut HidDevReaderWriter, mode: LightMode) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.light_mode = mode;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_light_brightness(rw: &mut HidDevReaderWriter) -> ProtoResult<u8> {
    let info = get_function_info(rw).await?;
    Ok(info.light_brightness)
}

pub async fn set_light_brightness(rw: &mut HidDevReaderWriter, brightness: u8) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.light_brightness = max(4, brightness);
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_light_speed(rw: &mut HidDevReaderWriter) -> ProtoResult<u8> {
    let info = get_function_info(rw).await?;
    Ok(info.light_speed)
}

pub async fn set_light_speed(rw: &mut HidDevReaderWriter, speed: u8) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.light_speed = max(4, speed);
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_light_reverse(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let info = get_function_info(rw).await?;
    Ok(info.light_reverse)
}

pub async fn set_light_reverse(rw: &mut HidDevReaderWriter, reverse: bool) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.light_reverse = reverse;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_rainbow_mode(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let info = get_function_info(rw).await?;
    Ok(info.rainbow_mode)
}

pub async fn set_rainbow_mode(rw: &mut HidDevReaderWriter, rainbow: bool) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.rainbow_mode = rainbow;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_sleep_time(rw: &mut HidDevReaderWriter) -> ProtoResult<u16> {
    let info = get_function_info(rw).await?;
    Ok(info.sleep_time)
}

pub async fn set_sleep_time(rw: &mut HidDevReaderWriter, sleep_time: u16) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.sleep_time = sleep_time;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_swap_wasd(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let info = get_function_info(rw).await?;
    Ok(info.swap_wasd)
}

pub async fn set_swap_wasd(rw: &mut HidDevReaderWriter, swap: bool) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.swap_wasd = swap;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_all_key_punchless(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let info = get_function_info(rw).await?;
    Ok(info.all_key_punchless)
}

pub async fn set_all_key_punchless(rw: &mut HidDevReaderWriter, punchless: bool) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.all_key_punchless = punchless;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_lock_win(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let info = get_function_info(rw).await?;
    Ok(info.lock_win)
}

pub async fn set_lock_win(rw: &mut HidDevReaderWriter, lock: bool) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.lock_win = lock;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_polling_rate(rw: &mut HidDevReaderWriter) -> ProtoResult<PollingRate> {
    let info = get_function_info(rw).await?;
    Ok(info.polling_rate)
}

pub async fn set_polling_rate(rw: &mut HidDevReaderWriter, rate: PollingRate) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.polling_rate = rate;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_mac_mode(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let info = get_function_info(rw).await?;
    Ok(info.mac_mode)
}

pub async fn set_mac_mode(rw: &mut HidDevReaderWriter, mac_mode: bool) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.mac_mode = mac_mode;
    set_function_info(rw, &info).await?;
    Ok(())
}

pub async fn get_smart_speed(rw: &mut HidDevReaderWriter) -> ProtoResult<bool> {
    let info = get_function_info(rw).await?;
    Ok(info.enable_smart_speed)
}

pub async fn set_smart_speed(rw: &mut HidDevReaderWriter, smart_speed: bool) -> ProtoResult<()> {
    let mut info = get_function_info(rw).await?;
    info.enable_smart_speed = smart_speed;
    set_function_info(rw, &info).await?;
    Ok(())
}
