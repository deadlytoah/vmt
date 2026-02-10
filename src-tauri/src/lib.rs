use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamError;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, thiserror::Error, serde::Serialize)]
enum VMTError {
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

fn cpal_config_to_hound(source: &cpal::StreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: source.channels,
        sample_rate: source.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    }
}

#[tauri::command]
fn start_recording(stream: tauri::State<'_, cpal::Stream>) -> Result<(), VMTError> {
    stream.play()?;
    Ok(())
}

#[tauri::command]
async fn stop_recording(
    config: tauri::State<'_, cpal::StreamConfig>,
    stream: tauri::State<'_, cpal::Stream>,
    buffer: tauri::State<'_, Arc<Mutex<Vec<f32>>>>,
) -> Result<String, VMTError> {
    stream.pause()?;
    {
        let mut writer = hound::WavWriter::create("../voice.wav", cpal_config_to_hound(&config))?;
        let mut v = buffer.lock().expect("failed locking mutex");
        for b in v.iter() {
            writer.write_sample((b.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
        }
        writer.finalize()?;
        v.clear();
    }
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    Ok("transcript".to_owned())
}

fn handle_error(err: StreamError) {}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let host = cpal::default_host();
    let device = host.default_input_device().expect("no input devices found");
    let config = device
        .default_input_config()
        .expect("no input config found")
        .config();
    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let v = Arc::clone(&buffer);
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                v.lock().expect("mutex lock").extend(data);
            },
            handle_error,
            None,
        )
        .expect("error building audio stream");
    // start with a paused stream
    stream.pause().expect("error pausing audio stream");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(config)
        .manage(stream)
        .manage(buffer)
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
