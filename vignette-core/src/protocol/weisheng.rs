use std::io::Cursor;

use binrw::{BinRead, BinWrite, binrw};

pub const WS_REPORT_ID: u8 = 4;

/// Data frame in the weisheng protocol.
///
/// The length of the frame is 64 bytes at most for a wired connection. For a
/// wireless connection, the maximum length is 32 bytes. If the data to be
/// transmitted is too long, it will be fragmented into multiple frames.
#[binrw]
#[brw(magic(b"\0\0"), little)]
#[derive(Debug)]
pub struct WsFrame {
    /// Command code
    cmd: WsCmd,
    /// Length of the payload
    #[br(calc = cmd.resp_len(), pad_after = 1)]
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
    pub fn new(conn: WsConnType, cmd: WsCmd, data: Option<&[u8]>) -> Self {
        let data_len = data.map_or_default(|d| d.len());
        let max_len = match conn {
            WsConnType::Wired => 56,
            WsConnType::Wireless => 24,
        };
        if data_len > max_len {
            panic!(
                "Data fragmentation is not supported yet. Data length: {}, max length: {}",
                data_len, max_len
            );
        }

        let mut frame_data = Vec::with_capacity(data_len);
        if let Some(data) = data {
            frame_data.extend_from_slice(data);
        }

        WsFrame {
            cmd,
            offset: 0,
            data: frame_data,
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut cursor = Cursor::new(bytes);
        WsFrame::read(&mut cursor).expect("Failed to read WsFrame from bytes")
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::with_capacity(64));
        self.write(&mut buf)
            .expect("Failed to write WsFrame to bytes");
        buf.into_inner()
    }
}

pub enum WsConnType {
    Wired,
    Wireless,
}

/// Command code in the weisheng protocol.
#[binrw]
#[derive(Debug)]
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
}

impl WsCmd {
    /// Returns the expected length of the payload in the response frame.
    fn resp_len(&self) -> u8 {
        match self {
            WsCmd::GetBatteryLevel => 2,
            _ => 56,
        }
    }
}
