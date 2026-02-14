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
- [x] Update the frontend UI elements for auto-scroll
- [x] Auto-scroll to latest text

## Stage 3: Energy-threshold VAD
- [x] Calibrate noise floor from ~300ms of mic input during setup
- [ ] Compute per-frame RMS energy with adaptive noise floor (EMA)
- [ ] Add hangover timer and switch chunk flushing from fixed interval to silence detection
