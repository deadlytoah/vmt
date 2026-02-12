mod error;
mod transcribe;

use crate::error::VMTError;
use crate::transcribe::{Transcriber, WhisperService};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamError;
use std::env;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tauri::{async_runtime::spawn, Emitter, Manager};
use tokio::time::Duration;

const FRAME_MS: f32 = 0.02;
const FRAME_COUNT: usize = 100;
const MIN_BUFSIZE: usize = 1024 * 1024 * 4;
const POLLING_INTERVAL: Duration = Duration::from_millis(20);

fn cpal_config_to_hound(source: &cpal::StreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: source.channels,
        sample_rate: source.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    }
}

fn read_rb(
    ac: &mut Vec<f32>,
    consumer: &mut rtrb::Consumer<f32>,
    frame_size: usize,
) -> Result<(), VMTError> {
    let rc = consumer.read_chunk(frame_size)?;
    let (a, b) = rc.as_slices();
    ac.extend(a);
    ac.extend(b);
    rc.commit_all();
    Ok(())
}

async fn transcribe_frame(
    config: &cpal::StreamConfig,
    transcriber: &WhisperService,
    sample: &[f32],
) -> Result<String, VMTError> {
    let wav: Vec<u8> = {
        let mut cursor = Cursor::new(Vec::with_capacity(MIN_BUFSIZE));
        let mut writer = hound::WavWriter::new(&mut cursor, cpal_config_to_hound(config))?;
        for c in sample {
            writer.write_sample((c.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
        }
        writer.finalize()?;
        cursor.into_inner()
    };
    transcriber.transcribe(wav).await
}

#[tauri::command]
fn start_recording(stream: tauri::State<'_, cpal::Stream>) -> Result<(), VMTError> {
    stream.play()?;
    Ok(())
}

#[tauri::command]
async fn stop_recording(
    stream: tauri::State<'_, cpal::Stream>,
    transcriptions: tauri::State<'_, Arc<Mutex<Vec<String>>>>,
) -> Result<String, VMTError> {
    stream.pause()?;
    let mut tv = transcriptions.lock().expect("failed to lock mutex");
    let t = tv.join(" ");
    tv.clear();
    Ok(t)
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

            let api_key = env::var("OPENAI_API_KEY").map_err(|_| VMTError::Transcript {
                message: "API key not set in environment".into(),
            })?;
            let transcriber = WhisperService::new(&api_key);

            let frame_size = (FRAME_MS * (config.sample_rate as f32)) as usize;
            let transcriptions = Arc::new(Mutex::new(Vec::with_capacity(MIN_BUFSIZE)));
            let mut ac: Vec<f32> = Vec::with_capacity(MIN_BUFSIZE);
            let tv_cloned = Arc::clone(&transcriptions);
            let config_cloned = config.clone();
            let cs_task_handle = spawn(async move {
                let mut consumer = consumer;
                loop {
                    if consumer.slots() < frame_size {
                        tokio::time::sleep(POLLING_INTERVAL).await;
                    } else if let Err(e) = read_rb(&mut ac, &mut consumer, frame_size) {
                        eprintln!("Error reading from ring buffer: {}", e);
                    } else if ac.len() > frame_size * FRAME_COUNT {
                        match transcribe_frame(&config_cloned, &transcriber, &ac).await {
                            Ok(transcription) => tv_cloned
                                .lock()
                                .expect("failed to lock mutex")
                                .push(transcription),
                            Err(e) => eprintln!("Error transcribing frame: {}", e),
                        }
                        ac.clear();
                    } else {
                        // Wait for more data in the sample for transcription
                    }
                }
            });

            // Tie the lifecycle of the background consumer task to
            // that of the app.
            spawn(async move {
                let _ = cs_task_handle.await;
                std::process::exit(1);
            });

            app.manage(stream);
            app.manage(transcriptions);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![start_recording, stop_recording])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
