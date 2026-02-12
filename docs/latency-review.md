# Latency Review

Latency issues identified in the codebase and their resolution status.

## V2

### Logging in audio callback
The ring buffer write error path must avoid allocations and locks. Currently the callback silently drops errors with a TODO comment.

**Fix:** Use an `AtomicBool` flag set in the callback, checked from a non-RT thread.

### Native sample rate sent to Whisper
Device sample rates vary across platforms (44.1/48 kHz typical). Whisper operates at 16 kHz internally, so sending native-rate audio is ~3× the payload for zero quality benefit. On any reasonable CPU, resampling a few seconds of audio is sub-millisecond; the network savings from a smaller payload should dominate.

**Fix:** Resample to 16 kHz in the consumer before encoding chunks.

## V1

### Pre-allocate audio buffer
**Status:** Fixed

`Vec::<f32>::new()` started at capacity 0, causing repeated reallocations on the audio callback thread. Now pre-allocated with `with_capacity`.

### Mutex on audio thread
**Status:** Fixed

The cpal callback locks a `Mutex` shared with `stop_recording`. In V1 the stream is paused before encoding so there's no contention, but for streaming the audio thread would block whenever the consumer holds the lock.

**Fix:** Replace `Arc<Mutex<Vec<f32>>>` with a lock-free ring buffer (`rtrb`).

### Lock held during WAV encoding
**Status:** Fixed

`stop_recording` holds the mutex while iterating every sample and writing WAV output. For streaming, this is a long hold that blocks the audio callback.

**Fix:** Decouple capture and encoding — consumer reads from a ring buffer independently.

### HTTP client created per request
**Status:** Fixed

`reqwest::Client::new()` was called inside every `transcribe()` invocation, discarding connection pooling and TLS session reuse. For streaming with many short chunks this adds a full TLS handshake per request.

**Fix:** Client is now stored in `WhisperService` and created once at startup.

### WAV encoding writes to disk
**Status:** Fixed

`hound::WavWriter::create` wrote to a file. Now encodes to `Cursor<Vec<u8>>` in memory via `hound::WavWriter::new`.
