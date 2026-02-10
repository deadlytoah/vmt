# Voice Memo Transcriber — Functional Design

## Timeline
- **Started:** 2026-02-10
- **Target:** 2026-02-17

## Overview
Desktop app that captures microphone audio and transcribes it.

## V1 Scope (1 week)
- Press a button to start/stop recording from the system microphone.
- On stop, transcribe the recorded audio and display the transcript.

## UI Design
Top-down split layout. Top half: a prominent record/stop button with a live elapsed-time counter. Bottom half: a scrollable transcript pane that populates after transcription completes. Extends naturally to streaming live text, in-place editing, and a sidebar for memo history.

## Error Handling (V1)
- If the audio stream encounters an error during recording, display an error message and ask the user to restart the app.

## Remaining Scope
- Save the transcript to a local file.
- In-app transcript editing.
- Live streaming transcription during recording.
- LLM summarisation of transcripts.
- Multiple memo management / history.
