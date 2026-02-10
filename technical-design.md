# Voice Memo Transcriber — Technical Design

## Stack
- **Desktop shell:** Tauri v2
- **Backend language:** Rust
- **Frontend framework:** Next.js (TypeScript)
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
3. Whisper returns transcript text — Rust passes it to frontend via Tauri event.
4. Frontend displays the transcript.

## Remaining
- Save transcript to local file via Tauri command.
- Streaming transcription (chunked audio while recording).
- LLM summarisation via OpenAI Chat Completions API.
- Persistent storage / memo history.
