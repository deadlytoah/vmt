# V2 Research Findings

## Lock-Free Ring Buffer

`rtrb` recommended. Purpose-built for real-time audio with strict wait-free, zero-alloc guarantees. Batch write via `write_chunk_uninit()`, zero-copy read via `read_chunk().as_slices()`, clean shutdown via `is_abandoned()`. `ringbuf` is more popular but lacks the same real-time safety contract.

## cpal Callback Sizes

`BufferSize::Fixed(n)` is a hint, not a guarantee. Callback slice length is variable and platform-dependent. The consumer must accumulate samples and decide when to flush.

## Whisper Chunk Boundary Artefacts

Whisper produces garbled output when audio is split mid-word: fragmented words, broken punctuation, hallucinated text. Mitigations:
- **VAD** — cut at silence boundaries to avoid mid-word splits.
- **Prompt conditioning** — pass previous transcript tail as Whisper's `prompt` (max 224 tokens) for continuity.
- **Overlapping windows** — send overlapping chunks then deduplicate. Increases cost and complexity.

## OpenAI Realtime API Downsides

1. **~10–20x more expensive.** $0.05–0.13/min vs $0.006/min for REST Whisper.
2. **WebSocket complexity.** 60-min session cap, silent disconnections, no proper CLOSE frames, manual reconnection.
3. **Hard vendor lock-in.** Proprietary event protocol with no drop-in alternatives.
4. **Immature.** GA since August 2025. Frequent reports of connection issues.
5. **Restrictive format.** Requires PCM16 at exactly 24 kHz mono; needs a real-time resampler.
6. **Hard to test.** No mock server exists. REST is trivially mockable.

## VAD Approach: Energy-Threshold vs Silero VAD

### Energy-Threshold
- Compute RMS energy per frame, compare against a threshold to classify speech vs silence.
- Stateless (or simple moving average). Works at any sample rate, any frame size.
- ~85–92% accuracy in a quiet room. Degrades with background noise — HVAC, keyboard clicks, and breathing register as speech.
- <1 µs per frame. Zero dependencies, ~80–120 LOC pure Rust, zero binary size impact.
- Requires adaptive threshold tracking the noise floor plus a hangover timer (200–500 ms silence before cutting).

### Silero VAD
- CNN + LSTM neural network running via ONNX Runtime. Outputs speech probability per 32 ms frame.
- Stateful — hidden states carry temporal context. Requires 16 kHz input (resampling needed if cpal default differs).
- ~97% ROC-AUC on multi-domain benchmarks. 4x fewer errors than WebRTC VAD at 5% FPR. Robust to non-speech sounds.
- <1 ms per frame (ONNX inference on CPU). Adds `ort` + model file dependency, +15–30 MB to binary (ONNX Runtime dylib dominates).
- Rust crates available: `voice_activity_detector` (cleanest API), `silero-vad-rs`, `silero-vad-rust`.

### Comparison

| | Energy-Threshold | Silero VAD |
|---|---|---|
| Accuracy (quiet) | ~85–92% | ~97% |
| Noisy environments | Poor — can't distinguish noise from speech | Good — trained on diverse noise |
| Per-frame latency | <1 µs | <1 ms |
| Dependencies | None | `ort`, ONNX model, possibly `rubato` for resampling |
| Binary size impact | 0 | +15–30 MB |
| Implementation | ~80–120 LOC pure Rust | ~50–80 LOC with `ort`, or drop-in crate |
| Build complexity | None | C compiler for ONNX Runtime; linking strategy choice |

### Decision

Energy-threshold. Zero dependencies, negligible overhead, and sufficient accuracy for chunking — Whisper is robust to imprecise boundaries. Demonstrates low-level Rust skill. Upgrade path to Silero (or `webrtc-vad` as a middle ground) requires no architectural change — same frame-in, bool-out interface.

## Adaptive Threshold

### Problem
A fixed RMS threshold breaks when ambient noise changes (fan turns on, user moves rooms, mic gain changes).

### Algorithm
The EMA itself is a linear first-order IIR filter, but the overall system is non-linear: the noise floor only updates during frames classified as silence, and that classification depends on the noise floor. The output gates its own state update.

1. **Noise floor EMA.** During silence frames: `noise_floor = α * frame_rms + (1 - α) * noise_floor`. Small α (0.01–0.05) prevents chasing transient spikes.
2. **Speech threshold.** `frame_rms > noise_floor * multiplier` (multiplier 2–4x). Frame is speech if above.
3. **Hangover timer.** After speech drops below threshold, require 200–500 ms of consecutive silence before declaring a chunk boundary. Prevents cutting during brief inter-word pauses.

### Calibration Seed
The stream is built and immediately paused during `setup()`. Run the mic for ~300 ms before pausing to capture ambient noise and seed the noise floor. Use a Tokio async task to avoid blocking app init. Clear the recording buffer after calibration; the noise floor estimate lives in its own state.

### Latch-Up Risk
If sustained noise is misclassified as speech, the noise floor never updates. Mitigations:
- **Slow unconditional decay.** Always nudge noise floor upward slightly, even during speech frames.
- **Max speech duration cap.** Force recalibration if speech persists beyond ~30 s without a silence gap.

### Parameters
- `α` — EMA smoothing factor (noise floor adaptation speed)
- `multiplier` — speech threshold relative to noise floor
- `hangover_ms` — silence duration before cutting (200–500 ms)
- `calibration_ms` — initial quiet capture for seeding (~300 ms)

## Decision

VAD + chunked REST Whisper. Simpler, cheaper, testable, and demonstrates more engineering than offloading to a black-box WebSocket.
