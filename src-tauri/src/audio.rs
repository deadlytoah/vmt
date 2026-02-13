use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::StreamError;
use tauri::Emitter;

use crate::error::VMTError;

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
    // start with a paused stream
    stream.pause()?;
    Ok(stream)
}
