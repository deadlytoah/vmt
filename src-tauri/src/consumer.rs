use tauri::async_runtime::spawn;

use crate::error::VMTError;
use crate::transcribe::{Transcriber, WhisperService};
use crate::MIN_BUFSIZE;
use std::io::Cursor;
use tauri::Emitter;
use tokio::time::Duration;

const FRAME_COUNT: usize = 100;
const FRAME_MS: f32 = 0.02;
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

async fn transcribe_and_emit(
    app_handle: &tauri::AppHandle,
    config: &cpal::StreamConfig,
    transcriber: &WhisperService,
    ac: &[f32],
) -> Result<(), VMTError> {
    let transcription = transcribe_frame(config, transcriber, ac).await?;
    app_handle.emit("partial-transcript", transcription)?;
    Ok(())
}

pub fn run_loop(
    app_handle: tauri::AppHandle,
    mut consumer: rtrb::Consumer<f32>,
    config: cpal::StreamConfig,
    mut flush_rx: tokio::sync::mpsc::Receiver<tokio::sync::oneshot::Sender<()>>,
    transcriber: WhisperService,
) {
    let frame_size = (FRAME_MS * (config.sample_rate as f32)) as usize;
    let mut ac: Vec<f32> = Vec::with_capacity(MIN_BUFSIZE);
    let cs_task_handle = spawn(async move {
        loop {
            if let Ok(reply_tx) = flush_rx.try_recv() {
                let flush_frame_size = consumer.slots();
                if let Err(e) = read_rb(&mut ac, &mut consumer, flush_frame_size) {
                    eprintln!("Error reading from ring buffer: {}", e);
                } else if !ac.is_empty() {
                    if let Err(e) =
                        transcribe_and_emit(&app_handle, &config, &transcriber, &ac).await
                    {
                        eprintln!("Error while transcribing: {}", e);
                    }
                    ac.clear();
                }
                let _ = reply_tx.send(());
            } else if consumer.slots() < frame_size {
                tokio::time::sleep(POLLING_INTERVAL).await;
            } else if let Err(e) = read_rb(&mut ac, &mut consumer, frame_size) {
                eprintln!("Error reading from ring buffer: {}", e);
            } else if ac.len() > frame_size * FRAME_COUNT {
                if let Err(e) = transcribe_and_emit(&app_handle, &config, &transcriber, &ac).await {
                    eprintln!("Error while transcribing: {}", e);
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
}
