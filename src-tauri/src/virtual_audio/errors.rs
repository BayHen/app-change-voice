use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtualAudioError {
    NotImplemented,
    DriverNotInstalled,
    BridgeUnavailable,
    IncompatibleProtocol { expected: u32, actual: u32 },
    InvalidFormat(String),
    AccessDenied,
    Transport(String),
}

impl Display for VirtualAudioError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented => {
                formatter.write_str("VoiceMorph virtual driver is not implemented")
            }
            Self::DriverNotInstalled => {
                formatter.write_str("VoiceMorph virtual driver is not installed")
            }
            Self::BridgeUnavailable => {
                formatter.write_str("VoiceMorph audio bridge is unavailable")
            }
            Self::IncompatibleProtocol { expected, actual } => write!(
                formatter,
                "Bridge protocol mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidFormat(message) => {
                write!(formatter, "Unsupported bridge audio format: {message}")
            }
            Self::AccessDenied => {
                formatter.write_str("Access to the virtual audio driver was denied")
            }
            Self::Transport(message) => {
                write!(formatter, "Virtual audio transport failed: {message}")
            }
        }
    }
}

impl std::error::Error for VirtualAudioError {}
