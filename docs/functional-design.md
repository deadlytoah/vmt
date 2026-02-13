# Voice Memo Transcriber — Functional Design

## Timeline
- **V1:** 2026-02-10 → 2026-02-11 (complete)
- **V2:** 2026-02-11 → 2026-02-17

## Overview
Desktop app that captures microphone audio and transcribes it.

## V1 Scope (1 week)
- Press a button to start/stop recording from the system microphone.
- On stop, transcribe the recorded audio and display the transcript.

## UI Design
Top-down split layout. Top half: a prominent record/stop button with a live elapsed-time counter. Bottom half: a scrollable transcript pane that populates after transcription completes. Extends naturally to streaming live text, in-place editing, and a sidebar for memo history.

## Assumptions
- The app requires a network connection for the OpenAI Whisper API. Offline handling (e.g. disabling Start when no network is detected) is deferred to a future version.

## Error Handling (V1)
- If the audio stream encounters an error during recording, display an error message and ask the user to restart the app.

## V2 Scope — Live Streaming Transcription
- While recording, transcribe audio in chunks and stream partial results into the UI in real time.
- The transcript pane updates incrementally as text arrives, rather than appearing all at once after stop. Auto-scrolls to the latest text.

## Remaining Scope
- Save the transcript to a local file.
- In-app transcript editing.
- LLM summarisation of transcripts.
- Multiple memo management / history.
