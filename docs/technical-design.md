# Voice Memo Transcriber — Technical Design

## Stack
- **Desktop shell:** Tauri v2
- **Backend language:** Rust
- **Frontend framework:** React (TypeScript), Vite
- **Audio capture:** `cpal` crate for cross-platform microphone input
- **Audio encoding:** Encode captured PCM samples to WAV in Rust
- **Transcription:** `Transcriber` trait with OpenAI Whisper API as the default implementation
- **HTTP client:** `reqwest` crate for async API calls from Rust
- **IPC:** Tauri command/event bridge between Rust backend and Next.js frontend
- **Async runtime:** Tokio (provided by Tauri v2)

## Design Principles
- Design for low latency from the start: prefer zero-copy patterns, efficient algorithms, and minimal allocations so the architecture is ready for streaming without a rewrite.

## V1 Flow
1. User clicks Record — Tauri command starts `cpal` audio capture, buffering PCM in memory.
2. User clicks Stop — Rust encodes buffer to WAV, sends it to Whisper API.
3. Whisper returns transcript text — Rust returns it as the `stop_recording` command response. (Swap to Tauri event for streaming scope.)
4. Frontend displays the transcript.

## Error Handling (V1)
- The `cpal` error callback emits a Tauri event to the frontend.
- The frontend displays an error message and asks the user to restart the app.
- No automatic recovery or partial-transcription salvage in V1.

## V2 — Live Streaming Transcription

### Overview
Replace the batch record-then-transcribe flow with a pipeline that transcribes audio chunks while recording continues. Three concurrent stages: capture, encode+send, and UI update.

### Technical Changes

**1. Lock-free ring buffer (`rtrb` crate)**
Replace `Arc<Mutex<Vec<f32>>>` with an `rtrb` SPSC ring buffer. The
cpal callback produces into one end; a Tokio consumer task reads from
the other. Wait-free and zero-alloc on the hot path.

**2. Frame-aligned consumer with energy-threshold VAD**
Consumer polls the ring buffer in a loop, sleeping briefly when empty
to avoid starving the tokio thread pool. It reads `frame_size` samples
at a time and commits each frame immediately. Each frame is classified
as speech or silence via adaptive energy-threshold VAD:

    rms           = sqrt(mean(sample² for sample in frame))
    is_speech     = rms > noise_floor × threshold_ratio
    if !is_speech:
        noise_floor = α × rms + (1 − α) × noise_floor

Parameters: **α** (EMA smoothing, 0.001), **threshold_ratio**
(speech multiplier, 5.0). Noise floor seeded from ~1 s mic capture
at startup. Frames accumulate locally; a hangover timer
(200–500 ms silence) triggers a flush for transcription. The consumer
runs as a `tokio::spawn` task whose lifecycle is tied to the app — if
it exits for any reason (success, error, or panic), the app
terminates. The consumer must run for the app's entire lifetime; early
exit indicates a logic bug or fatal error.

**3. Tauri event streaming**
Each chunk's transcript is emitted as a Tauri event
(`transcript-chunk`) instead of returned as a command response. The
frontend listens and appends text incrementally.

### V2 Pipeline
1. User clicks Record — cpal callback writes raw PCM samples into a flat `f32` ring buffer.
2. Consumer reads fixed-size frames, runs VAD per frame, accumulates. On silence boundary, flushes chunk for transcription.
3. Each transcript is emitted as a `transcript-chunk` Tauri event.
4. Frontend appends each chunk to the transcript pane.
5. User clicks Stop — consumer flushes remaining accumulator, transcribes final chunk.

### Deferred to Post-V2
- **Resample to 16 kHz before sending to Whisper** — device sample rates vary across platforms (44.1/48 kHz typical). Whisper operates at 16 kHz internally, so native-rate audio is ~3× the payload for zero quality benefit. Resampling in the consumer also normalises frame math (`FRAME_SIZE = 320` = 20 ms).
- **Latch-up prevention** — slow unconditional noise floor decay during speech frames. V2 targets quiet-room use where latch-up is unlikely.
- **Max speech duration cap** — force noise floor recalibration after ~30s continuous speech. Related to latch-up; defer together.
- **Minimum chunk size policy** — natural speech pauses produce reasonable chunks; revisit if short utterances cause excessive API calls.
- **Prompt conditioning** — pass previous transcript tail as Whisper's `prompt` parameter (max 224 tokens) for cross-chunk continuity. Optimization, not required for core streaming.

### Risks
- Lock-free ring buffer correctness — mitigated by using `rtrb`, a proven real-time audio crate.
- Chunk boundary artefacts — energy-threshold VAD reduces risk; prompt conditioning can further improve continuity post-V2.
- Network latency variance — slow API responses could cause chunks to back up. Need backpressure or bounded concurrency.

## Remaining
- Save transcript to local file via Tauri command.
- LLM summarisation via OpenAI Chat Completions API.
- Persistent storage / memo history.
