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
the other. Wait-free and zero-alloc on the hot path. `is_abandoned()`
on both ends detects shutdown.

**2. Frame-aligned consumer with energy-threshold VAD**
Consumer reads `frame_size` samples at a time from the ring buffer
and commits each frame immediately. Adaptive RMS classifies each
frame as speech or silence. Frames accumulate locally; a hangover
timer (200–500 ms silence) triggers a flush for transcription. Noise
floor seeded via ~300 ms mic capture at startup. Pure Rust, zero
dependencies.

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

### Risks
- Lock-free ring buffer correctness — mitigated by using `rtrb`, a proven real-time audio crate.
- Chunk boundary artefacts — energy-threshold VAD reduces risk; prompt conditioning (passing the previous transcript tail as Whisper's `prompt` parameter, max 224 tokens) further improves continuity.
- Network latency variance — slow API responses could cause chunks to back up. Need backpressure or bounded concurrency.

## Remaining
- Save transcript to local file via Tauri command.
- LLM summarisation via OpenAI Chat Completions API.
- Persistent storage / memo history.
