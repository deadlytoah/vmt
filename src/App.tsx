import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

function App() {
  const [elapsed, setElapsed] = useState("00:00");
  const [transcript, setTranscript] = useState("");
  const [isRecording, setIsRecording] = useState(false);
  const [timer, setTimer] = useState<ReturnType<typeof setInterval> | null>(null);
  const [information, setInformation] = useState("Press Record to begin");
  const [error, setError] = useState("");

  function updateTick(startTime: number) {
    const now = Math.floor((Date.now() - startTime) / 1000);
    const secs = now % 60;
    const mins = Math.floor(now / 60);
    setElapsed(`${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`);
  }

  async function startRecording() {
    try {
      await invoke("start_recording", {});
      setInformation("Recording in progress.");
      setTranscript("");
      setIsRecording(true);
      setTimer(setInterval(updateTick, 200, Date.now()));
    } catch (e: any) {
      if ("PlayStream" in e) {
        setError(`Error starting audio: ${e["PlayStream"]["message"]}`);
      }
    }
  }

  async function stopRecording() {
    function resetUI() {
      setInformation("");
      setIsRecording(false);
      if (timer != null) {
        clearInterval(timer);
        setTimer(null);
        setElapsed("00:00");
      }
    }

    setInformation("Transcript will appear when ready.");
    try {
      await invoke("stop_recording", {});
      resetUI();
    } catch (e: any) {
      if ("StopStream" in e) {
        setError(`Error stopping audio: ${e["StopStream"]["message"]}`);
      } else if ("Hound" in e) {
        setError(`Error encoding audio: ${e["Hound"]["message"]}`);
        resetUI();
      } else if ("Transcript" in e) {
        setError(`Transcription error: ${e["Transcript"]["message"]}`);
        resetUI();
      }
    }
  }

  function appendTranscript(partial: string) {
    setTranscript(prev => prev === "" ? partial : prev + " " + partial);
  }

  useEffect(() => {
    const unlisten = listen<string>("partial-transcript", (event) => {
      appendTranscript(event.payload);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  useEffect(() => {
    const unlisten = listen<string>("recording-error", (event) => {
      setError(`Error: ${event.payload}. Please restart the client.`);
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  return (
    <main className="container">
      <form
        className="row"
        onSubmit={async (e) => {
          e.preventDefault();
          if (isRecording) {
            await stopRecording();
          } else {
            await startRecording();
          }
        }}
      >
        <section className="controls">
          <button className="record-btn" type="submit">{isRecording ? "Stop" : "Record"}</button>
          <span className="elapsed">{elapsed}</span>
        </section>
      </form>
      <section className="transcript-pane">
        <p className="error">{error}</p>
        <p className="information">{information}</p>
        <p className="transcript-text">{transcript}</p>
      </section>
    </main>
  );
}

export default App;
