#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum VMTError {
    #[error("failed to play stream: {message}")]
    PlayStreamError { message: String },
    #[error("failed to stop stream: {message}")]
    StopStreamError { message: String },
    #[error("failed to encode audio: {message}")]
    HoundError { message: String },
    #[error("failed to transcript stream: {message}")]
    TranscriptError { message: String },
}

impl From<cpal::PlayStreamError> for VMTError {
    fn from(source: cpal::PlayStreamError) -> Self {
        Self::PlayStreamError {
            message: source.to_string(),
        }
    }
}

impl From<cpal::PauseStreamError> for VMTError {
    fn from(source: cpal::PauseStreamError) -> Self {
        Self::StopStreamError {
            message: source.to_string(),
        }
    }
}

impl From<hound::Error> for VMTError {
    fn from(source: hound::Error) -> Self {
        Self::HoundError {
            message: source.to_string(),
        }
    }
}

impl From<reqwest::Error> for VMTError {
    fn from(source: reqwest::Error) -> Self {
        Self::TranscriptError {
            message: source.to_string(),
        }
    }
}
