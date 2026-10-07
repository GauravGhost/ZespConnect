use serde::{ser::SerializeStruct, Serialize, Serializer};

/// Unified error for every command. Serialises to `{ code, message }` so the
/// frontend can show a readable toast and branch on the `code` when needed.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("adb was not found. Install Android platform-tools or set its path in Settings.")]
    AdbMissing,

    #[error("scrcpy was not found. Install scrcpy to use mirroring.")]
    ScrcpyMissing,

    #[error("No device connected")]
    NoDevice,

    #[error("Device is unauthorized. Check the prompt on your phone.")]
    DeviceUnauthorized,

    #[error("Device is offline")]
    DeviceOffline,

    #[error("{0}")]
    Invalid(String),

    #[error("{0}")]
    CommandFailed(String),

    #[error("Could not parse command output: {0}")]
    Parse(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("{0}")]
    Other(String),
}

impl AppError {
    /// Stable machine-readable code shared with the frontend.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::AdbMissing => "adb_missing",
            AppError::ScrcpyMissing => "scrcpy_missing",
            AppError::NoDevice => "no_device",
            AppError::DeviceUnauthorized => "device_unauthorized",
            AppError::DeviceOffline => "device_offline",
            AppError::Invalid(_) => "invalid",
            AppError::CommandFailed(_) => "command_failed",
            AppError::Parse(_) => "parse",
            AppError::Io(_) => "io",
            AppError::Other(_) => "other",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("code", self.code())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        AppError::Io(value.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(value: serde_json::Error) -> Self {
        AppError::Parse(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
