# V2 TODO

## Stage 1: Fixed-interval chunked transcription
- [x] Create reqwest client once in setup, store as managed state
- [x] Create the asynchronous consumer task
- [x] Consumer creates fixed-size frame, sending it to Whisper
- [x] Accumulate frames into chunk buffer, flush at fixed time interval, multiple times, collecting partial transcripts
- [x] Collect partial transcripts for display at the end of recording

## Stage 2: Streaming UI updates
- [x] On stop, flush remaining audio in ring buffer as a final chunk
- [x] Use Tauri events as method of transferring events to the frontent
- [x] Update transcript pane incrementally (append, not replace)
- [ ] Update the frontend UI elements for auto-scroll
- [ ] Auto-scroll to latest text

## Stage 3: Energy-threshold VAD
- [ ] Run mic for ~300ms during setup to capture ambient noise
- [ ] Read calibration samples directly from consumer before managing it as state
- [ ] Seed noise floor from calibration samples
- [ ] Implement RMS energy computation per frame
- [ ] Implement adaptive noise floor (EMA, updates only during silence)
- [ ] Implement hangover timer (200–500ms silence before cutting)
- [ ] Switch chunk flushing from fixed interval to hangover expiry
