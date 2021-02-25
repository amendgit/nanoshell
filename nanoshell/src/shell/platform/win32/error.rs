use super::util::hresult_description;
use std::fmt::Display;

#[derive(Debug, Clone)]
pub enum PlatformError {
    UnknownError,
    LaunchEngineFailure,
    SendMessageFailure { channel: String },
    HResult(u32),
}

pub type PlatformResult<T> = Result<T, PlatformError>;

impl Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformError::UnknownError => {
                write!(f, "Unknown Error")
            }
            PlatformError::SendMessageFailure { channel } => {
                write!(f, "Failed to send message on channel {}", channel)
            }
            PlatformError::LaunchEngineFailure => {
                write!(f, "Failed to launch Flutter engine")
            }
            PlatformError::HResult(hresult) => {
                write!(
                    f,
                    "Error 0x{:X} ({})",
                    hresult,
                    hresult_description(*hresult).unwrap_or("Unknown".into())
                )
            }
        }
    }
}

impl std::error::Error for PlatformError {}
