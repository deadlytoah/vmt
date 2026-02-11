mod error;
mod transcribe;

use crate::error::VMTError;
use crate::transcribe::{Transcriber, WhisperService};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamError;
use std::env;
use std::io::Cursor;
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
    rb: tauri::State<'_, Mutex<rtrb::Consumer<f32>>>,
) -> Result<String, VMTError> {
    stream.pause()?;

    let wav: Vec<u8> = {
        let mut cursor = Cursor::new(Vec::with_capacity(MIN_BUFSIZE));
        let mut writer = hound::WavWriter::new(&mut cursor, cpal_config_to_hound(&config))?;
        let mut v = rb.lock().expect("failed locking mutex");
        let slots = v.slots();
        let rc = v
            .read_chunk(slots)
            .expect("failed to read from ring buffer");
        let (a, b) = rc.as_slices();
        for c in a.iter() {
            writer.write_sample((c.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
        }
        for c in b.iter() {
            writer.write_sample((c.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
        }
        writer.finalize()?;
        rc.commit_all();
        cursor.into_inner()
    };
    let api_key = env::var("OPENAI_API_KEY").map_err(|_| VMTError::Transcript {
        message: "API key not set in environment".into(),
    })?;
    let transcriber = WhisperService::new(&api_key);
    transcriber.transcribe(wav).await
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
            let (mut producer, consumer) = rtrb::RingBuffer::<f32>::new(MIN_BUFSIZE);
            let stream = device.build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| match producer
                    .write_chunk(data.len())
                {
                    Ok(mut wc) => {
                        let (a, b) = wc.as_mut_slices();
                        a.copy_from_slice(&data[..a.len()]);
                        b.copy_from_slice(&data[a.len()..]);
                        wc.commit_all();
                    }
                    Err(_) => {
                        // TODO: add some visibility for this error
                    }
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
            app.manage(Mutex::new(consumer));
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
