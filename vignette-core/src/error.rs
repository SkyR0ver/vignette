use thiserror::Error;

pub use async_hid::{HidError, HidResult};
pub use binrw::BinResult;

pub type ProtoResult<T> = Result<T, ProtoError>;

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("{0}")]
    HidError(#[from] async_hid::HidError),
    #[error("Invalid report ID: {0}")]
    InvalidReportId(u8),
    #[error("{0}")]
    BadFrame(#[from] binrw::Error),
    #[error("Response mismatch")]
    ResponseMismatch,
}
