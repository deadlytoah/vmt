# V2 TODO

## Energy-threshold VAD
- [ ] Implement RMS energy computation per frame
- [ ] Implement adaptive noise floor (EMA, updates only during silence)
- [ ] Implement hangover timer (200–500ms silence before cutting)

## Calibration
- [ ] Run mic for ~300ms during setup to capture ambient noise
- [ ] Read calibration samples directly from consumer before managing it as state
- [ ] Seed noise floor from calibration samples

## Chunked transcription loop
- [ ] Implement frame-aligned consumer: read fixed-size frames, commit immediately
- [ ] Accumulate frames into chunk buffer, flush on hangover expiry
- [ ] Encode accumulated chunk to WAV in memory
- [ ] Create reqwest client once in setup, store as managed state
- [ ] Send chunk to Whisper REST API
- [ ] On stop, flush remaining audio in ring buffer as a final chunk

## Streaming UI updates
- [ ] Emit partial transcript to frontend via Tauri events as each chunk completes
- [ ] Update transcript pane incrementally (append, not replace)
- [ ] Auto-scroll to latest text
