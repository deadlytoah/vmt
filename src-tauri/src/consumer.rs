use tauri::async_runtime::spawn;

use crate::audio;
use crate::error::VMTError;
use crate::transcribe::{Transcriber, WhisperService};
use crate::MIN_BUFSIZE;
use std::io::Cursor;
use tauri::Emitter;
use tokio::time::Duration;

const SILENCE_THRESHOLD: usize = 10;
const FLUSH_THRESHOLD: usize = 250;
const FRAME_COUNT: usize = 100;
const FRAME_MS: f32 = 0.02;
const POLLING_INTERVAL: Duration = Duration::from_millis(20);

enum VadState {
    Silence(usize),
    Speech(usize),
    MaybeSilence(usize),
    MaybeFlush,
}

impl VadState {
    fn silence(&mut self) -> bool {
        match *self {
            VadState::Silence(count) => {
                if count + 1 > FLUSH_THRESHOLD {
                    eprintln!("Too much silence");
                    *self = VadState::Silence(1);
                    false
                } else {
                    *self = VadState::Silence(count + 1);
                    false
                }
            }
            VadState::Speech(_) => {
                *self = VadState::MaybeSilence(1);
                false
            }
            VadState::MaybeSilence(count) => {
                if count + 1 < SILENCE_THRESHOLD {
                    *self = VadState::MaybeSilence(count + 1);
                    false
                } else {
                    eprintln!("Legit silence - maybe flush");
                    *self = VadState::MaybeFlush;
                    true
                }
            }
            _ => panic!("invalid state transition"),
        }
    }

    fn speech(&mut self) -> bool {
        match *self {
            VadState::Silence(_) => {
                eprintln!("Silence broke");
                *self = VadState::Speech(1);
                false
            }
            VadState::Speech(count) => {
                if count + 1 > FLUSH_THRESHOLD {
                    eprintln!("Speech too long");
                    *self = VadState::MaybeFlush;
                    true
                } else {
                    *self = VadState::Speech(count + 1);
                    false
                }
            }
            VadState::MaybeSilence(_) => {
                eprintln!("Maybe silence?");
                *self = VadState::Speech(1);
                false
            }
            _ => panic!("invalid state transition"),
        }
    }

    fn flush(&mut self) {
        if let VadState::MaybeFlush = *self {
            eprintln!("flush");
            *self = VadState::Silence(1);
        } else {
            panic!("invalid state transition");
        }
    }

    fn no_flush(&mut self) {
        if let VadState::MaybeFlush = *self {
            eprintln!("no flush");
            *self = VadState::MaybeSilence(1);
        } else {
            panic!("invalid state transition");
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

pub fn read_rb(
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

pub fn clear_rb(consumer: &mut rtrb::Consumer<f32>) -> Result<(), VMTError> {
    let slots = consumer.slots();
    let rc = consumer.read_chunk(slots)?;
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
    mut noise_floor: f32,
) {
    let frame_size = (FRAME_MS * (config.sample_rate as f32)) as usize;
    let mut ac: Vec<f32> = Vec::with_capacity(MIN_BUFSIZE);
    let cs_task_handle = spawn(async move {
        let mut vad_state = VadState::Silence(0);
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
            } else {
                let frame_index = ac.len() - frame_size;
                let frame = &ac[frame_index..];
                let rms = audio::rms(frame);

                let flush = if audio::is_silence(noise_floor, rms) {
                    noise_floor = audio::update_noise_floor(noise_floor, rms);
                    vad_state.silence()
                } else {
                    vad_state.speech()
                };

                if flush && ac.len() > frame_size * FRAME_COUNT {
                    vad_state.flush();
                    if let Err(e) =
                        transcribe_and_emit(&app_handle, &config, &transcriber, &ac).await
                    {
                        eprintln!("Error while transcribing: {}", e);
                    }
                    ac.clear();
                } else if flush {
                    vad_state.no_flush();
                }
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
