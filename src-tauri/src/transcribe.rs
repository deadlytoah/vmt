use crate::error::VMTError;

pub trait Transcriber {
    async fn transcribe(&self, buffer: Vec<u8>) -> Result<String, VMTError>;
}

pub struct WhisperService {
    api_key: String,
}

impl WhisperService {
    pub fn new(api_key: &str) -> Self {
        WhisperService {
            api_key: api_key.to_owned(),
        }
    }
}

impl Transcriber for WhisperService {
    async fn transcribe(&self, buffer: Vec<u8>) -> Result<String, VMTError> {
        let client = reqwest::Client::new();
        let multipart = reqwest::multipart::Part::bytes(buffer)
            .file_name("memo.wav")
            .mime_str("audio/wav")?;
        let form = reqwest::multipart::Form::new()
            .text("model", "whisper-1")
            .part("file", multipart);
        let response = client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await?;
        let transcription: serde_json::Value = response.json().await?;

        if !transcription["error"].is_null() {
            Err(VMTError::TranscriptError {
                message: transcription["error"]["message"].to_string(),
            })
        } else {
            transcription["text"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| VMTError::TranscriptError {
                    message: "data format error".into(),
                })
        }
    }
}
