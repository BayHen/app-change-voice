use serde::Serialize;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceError {
    pub code: &'static str,
    pub message: String,
}

impl AudioDeviceError {
    pub fn backend(message: impl Into<String>) -> Self {
        Self {
            code: "backend_error",
            message: message.into(),
        }
    }

    pub fn device_not_found(id: &str) -> Self {
        Self {
            code: "device_not_found",
            message: format!("Audio device '{id}' is no longer available"),
        }
    }

    pub fn persistence(message: impl Into<String>) -> Self {
        Self {
            code: "persistence_error",
            message: message.into(),
        }
    }
}

impl Display for AudioDeviceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AudioDeviceError {}
