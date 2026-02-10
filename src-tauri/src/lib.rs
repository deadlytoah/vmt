mod error;

use crate::error::VMTError;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamError;
use std::env;
use std::io::Cursor;
use std::sync::Arc;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

const MIN_BUFSIZE: usize = 1024 * 1024 * 4;

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

    let wav: Vec<u8> = {
        let mut cursor = Cursor::new(Vec::with_capacity(MIN_BUFSIZE));
        let mut writer = hound::WavWriter::new(&mut cursor, cpal_config_to_hound(&config))?;
        let mut v = buffer.lock().expect("failed locking mutex");
        for b in v.iter() {
            writer.write_sample((b.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
        }
        writer.finalize()?;
        v.clear();
        cursor.into_inner()
    };

    let api_key = env::var("OPENAI_API_KEY").map_err(|_| VMTError::TranscriptError {
        message: "API key not set in environment".into(),
    })?;
    let client = reqwest::Client::new();
    let multipart = reqwest::multipart::Part::bytes(wav)
        .file_name("memo.wav")
        .mime_str("audio/wav")?;
    let form = reqwest::multipart::Form::new()
        .text("model", "whisper-1")
        .part("file", multipart);
    let response = client
        .post("https://api.openai.com/v1/audio/transcriptions")
        .header("Authorization", format!("Bearer {}", api_key))
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
            let buffer = Arc::new(Mutex::new(Vec::<f32>::with_capacity(MIN_BUFSIZE)));
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
