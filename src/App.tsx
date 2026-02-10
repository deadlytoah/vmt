import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [elapsed, setElapsed] = useState("00:00");
  const [transcript, setTranscript] = useState("");
  const [isRecording, setIsRecording] = useState(false);
  const [timer, setTimer] = useState<ReturnType<typeof setInterval> | null>(null);
  const [information, setInformation] = useState("Press Record to begin");

  function updateTick(startTime: number) {
    const now = Math.floor((Date.now() - startTime) / 1000);
    const secs = now % 60;
    const mins = Math.floor(now / 60);
    setElapsed(`${String(mins).padStart(2, "0")}:${String(secs).padStart(2, "0")}`);
  }

  async function startRecording() {
    await invoke("start_recording", {});
    setInformation("Recording in progress.");
    setTranscript("");
    setIsRecording(true);
    setTimer(setInterval(updateTick, 200, Date.now()));
  }

  async function stopRecording() {
    setInformation("Transcript will appear when ready.");
    setTranscript("");
    try {
      setTranscript(await invoke("stop_recording", {}));
      // TODO: handle error and display error message.
    } finally {
      setInformation("");
      setIsRecording(false);
      if (timer != null) {
        clearInterval(timer);
        setTimer(null);
        setElapsed("00:00");
      }
    }
  }

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
        <p className="information">{information}</p>
        <p className="transcript-text">{transcript}</p>
      </section>
    </main>
  );
}

export default App;
