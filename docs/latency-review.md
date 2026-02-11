# Latency Review

Issues identified in the V1 codebase that would require a rewrite for streaming scope.

## Pre-allocate audio buffer
**Status:** Fixed

`Vec::<f32>::new()` started at capacity 0, causing repeated reallocations on the audio callback thread. Now pre-allocated with `with_capacity`.

## Mutex on audio thread
The cpal callback locks a `Mutex` shared with `stop_recording`. In V1 the stream is paused before encoding so there's no contention, but for streaming the audio thread would block whenever the consumer holds the lock.

**Fix:** Replace `Arc<Mutex<Vec<f32>>>` with a lock-free ring buffer.

## Lock held during WAV encoding
`stop_recording` holds the mutex while iterating every sample and writing WAV output. For streaming, this is a long hold that blocks the audio callback.

**Fix:** Decouple capture and encoding — consumer reads from a ring buffer independently.

## HTTP client created per request
`reqwest::Client::new()` is called inside every `transcribe()` invocation, discarding connection pooling and TLS session reuse. For streaming with many short chunks this adds a full TLS handshake per request.

**Fix:** Create the client once (e.g. in `setup()`) and store it as managed state or inside `WhisperService`.

## WAV encoding writes to disk
**Status:** Fixed

`hound::WavWriter::create` wrote to a file. Now encodes to `Cursor<Vec<u8>>` in memory via `hound::WavWriter::new`.
