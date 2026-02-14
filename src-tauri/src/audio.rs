use crate::consumer;
use cpal::traits::DeviceTrait;
use cpal::StreamError;
use tauri::Emitter;
use tokio::time::Duration;

use crate::error::VMTError;

const ALPHA: f32 = 0.001;
const THRESHOLD_RATE: f32 = 5.0;
const CALIBRATION_FRAME_SIZE: usize = (0.3 * 48000f32) as usize;
const CALIBRATION_DURATION: Duration = Duration::from_millis(1000);

pub fn build_audio_pipeline(
    app_handle: tauri::AppHandle,
    device: cpal::Device,
    config: cpal::StreamConfig,
    mut producer: rtrb::Producer<f32>,
) -> Result<cpal::Stream, VMTError> {
    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &cpal::InputCallbackInfo| match producer.write_chunk(data.len()) {
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
    Ok(stream)
}

pub fn rms(samples: &[f32]) -> f32 {
    let sum = samples.iter().map(|f| f * f).sum::<f32>();
    let mean = sum / (samples.len() as f32);
    mean.sqrt()
}

pub fn calibrate(consumer: &mut rtrb::Consumer<f32>) -> Result<f32, VMTError> {
    // capture ambient noise to use as calibration samples
    std::thread::sleep(CALIBRATION_DURATION);
    let mut buffer = Vec::with_capacity(CALIBRATION_FRAME_SIZE);
    let slots = consumer.slots();
    consumer::read_rb(&mut buffer, consumer, slots)?;
    let seed = rms(&buffer);
    Ok(seed)
}

pub fn update_noise_floor(noise_floor: f32, frame_rms: f32) -> f32 {
    ALPHA * frame_rms + (1.0 - ALPHA) * noise_floor
}

pub fn is_silence(noise_floor: f32, frame_rms: f32) -> bool {
    frame_rms < noise_floor * THRESHOLD_RATE
}
