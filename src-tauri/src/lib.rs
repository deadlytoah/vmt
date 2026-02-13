mod audio;
mod consumer;
mod error;
mod transcribe;

use crate::error::VMTError;
use crate::transcribe::WhisperService;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::env;
use tauri::Manager;

const MIN_BUFSIZE: usize = 1024 * 1024 * 4;

#[tauri::command]
fn start_recording(stream: tauri::State<'_, cpal::Stream>) -> Result<(), VMTError> {
    stream.play()?;
    Ok(())
}

#[tauri::command]
async fn stop_recording(
    stream: tauri::State<'_, cpal::Stream>,
    flush_tx: tauri::State<'_, tokio::sync::mpsc::Sender<tokio::sync::oneshot::Sender<()>>>,
) -> Result<(), VMTError> {
    stream.pause()?;
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let _ = flush_tx.send(reply_tx).await;
    let _ = reply_rx.await;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_handle = app.handle().clone();
            let (producer, consumer) = rtrb::RingBuffer::<f32>::new(MIN_BUFSIZE);
            let host = cpal::default_host();
            let device = host
                .default_input_device()
                .ok_or("no input devices found")?;
            let config = device.default_input_config()?.config();
            let stream = audio::build_audio_pipeline(app_handle, device, config.clone(), producer)?;
            app.manage(stream);

            let api_key = env::var("OPENAI_API_KEY").map_err(|_| VMTError::Transcript {
                message: "API key not set in environment".into(),
            })?;
            let transcriber = WhisperService::new(&api_key);

            let app_handle = app.handle().clone();
            let (flush_tx, flush_rx) =
                tokio::sync::mpsc::channel::<tokio::sync::oneshot::Sender<()>>(1);

            consumer::run_loop(app_handle, consumer, config, flush_rx, transcriber);

            app.manage(flush_tx);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
