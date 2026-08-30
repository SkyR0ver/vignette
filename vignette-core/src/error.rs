use std::fmt::Display;

pub use async_hid::{HidError, HidResult};
pub use binrw::BinResult;
pub type BinError = binrw::Error;

pub type ProtoResult<T> = Result<T, ProtoError>;

#[derive(Debug)]
pub enum ProtoError {
    HidError(HidError),
    InvalidReportId(u8),
    BadFrame(BinError),
    ResponseMismatch,
}

impl Display for ProtoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtoError::HidError(e) => Display::fmt(e, f),
            ProtoError::InvalidReportId(id) => write!(f, "Invalid report ID: {}", id),
            ProtoError::BadFrame(e) => Display::fmt(e, f),
            ProtoError::ResponseMismatch => write!(f, "Response mismatch"),
        }
    }
}

impl std::error::Error for ProtoError {}

impl From<HidError> for ProtoError {
    fn from(value: HidError) -> Self {
        ProtoError::HidError(value)
    }
}

impl From<BinError> for ProtoError {
    fn from(value: BinError) -> Self {
        ProtoError::BadFrame(value)
    }
}
