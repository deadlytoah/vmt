# Voice Memo Transcriber

Desktop app that captures microphone audio and transcribes it in real time using the OpenAI Whisper API.

Built with Tauri v2, Rust, and React (TypeScript).

## Features

### Live streaming transcription
Audio is transcribed in chunks while recording. Partial results stream into the UI incrementally, rather than appearing all at once after stop.

### Voice Activity Detection (VAD)
An energy-threshold VAD with adaptive noise floor drives chunking decisions:

- **RMS energy detection** — each 20 ms frame is classified as speech or silence based on its energy relative to the ambient noise floor.
- **Adaptive noise floor** — an exponential moving average tracks background noise, so the threshold adjusts to the environment. A calibration phase at startup seeds the initial estimate.
- **Three-state FSM** — a `Silence → Speech → MaybeSilence` state machine with a hangover timer prevents premature cutoffs mid-sentence. Chunks flush at natural silence boundaries.
- **Minimum chunk enforcement** — chunks shorter than 3 seconds are skipped to avoid sending fragments that produce poor transcriptions.

### Lock-free audio pipeline
The audio callback is allocation-free and lock-free:

- **SPSC ring buffer** (`rtrb`) — the `cpal` callback writes PCM samples directly into a lock-free ring buffer. The consumer reads from the other end on an async task.
- **Zero-copy writes** — the callback uses `write_chunk` with two-slice unwrap to handle circular buffer wraparound without copying.
- **No allocations on the hot path** — the audio callback does no heap allocation, mutex locking, or syscalls.

## Architecture

```
Microphone → cpal callback → Ring Buffer → Consumer task → Whisper API
                (lock-free)    (rtrb SPSC)    (tokio)        (reqwest)
                                                  ↓
                                            Tauri event → React UI
```

The consumer task polls the ring buffer at 50 Hz, feeds frames to the VAD, accumulates speech audio, and flushes complete chunks as WAV to the Whisper API. Transcription results are emitted as Tauri events that the React frontend subscribes to.

## Prerequisites

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/tools/install)
- [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```
