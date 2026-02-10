use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamError;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or("no input devices found")?;
            let config = device.default_input_config()?.config();
            let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
            let v = Arc::clone(&buffer);
            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    v.lock().expect("mutex lock").extend(data);
                },
                move |err: StreamError| {
                    let _ = app_handle
                        .emit("recording-error", err.to_string())
                        .inspect_err(|e| {
                            eprintln!("{}", e);
                        });
                },
                None,
            )?;
            // start with a paused stream
            stream.pause()?;

            app.manage(config);
            app.manage(stream);
            app.manage(buffer);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
