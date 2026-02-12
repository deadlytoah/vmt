#[derive(Debug, thiserror::Error, serde::Serialize)]
pub enum VMTError {
    #[error("failed to play stream: {message}")]
    PlayStream { message: String },
    #[error("failed to stop stream: {message}")]
    StopStream { message: String },
    #[error("failed to encode audio: {message}")]
    Hound { message: String },
    #[error("failed to transcript stream: {message}")]
    Transcript { message: String },
    #[error("failed ring buffer chunk operation: {message}")]
    RtrbChunk { message: String },
}

impl From<cpal::PlayStreamError> for VMTError {
    fn from(source: cpal::PlayStreamError) -> Self {
        Self::PlayStream {
            message: source.to_string(),
        }
    }
}

impl From<cpal::PauseStreamError> for VMTError {
    fn from(source: cpal::PauseStreamError) -> Self {
        Self::StopStream {
            message: source.to_string(),
        }
    }
}

impl From<hound::Error> for VMTError {
    fn from(source: hound::Error) -> Self {
        Self::Hound {
            message: source.to_string(),
        }
    }
}

impl From<reqwest::Error> for VMTError {
    fn from(source: reqwest::Error) -> Self {
        Self::Transcript {
            message: source.to_string(),
        }
    }
}

impl From<rtrb::chunks::ChunkError> for VMTError {
    fn from(source: rtrb::chunks::ChunkError) -> Self {
        Self::RtrbChunk {
            message: source.to_string(),
        }
    }
}
