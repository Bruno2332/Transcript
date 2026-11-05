import { useEffect, useRef, useState } from "react";
import recordingService from "../services/recordingService";
import errorMessage from "../utils/errorMessage";

type Status = "idle" | "recording" | "paused" | "finished";
interface Props {
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
  onFinished: (path: string) => void;
}

export default function Recorder({
  disabled,
  onBusyChange,
  onFinished,
}: Props) {
  const [status, setStatus] = useState<Status>("idle");
  const [pending, setPending] = useState(false);
  const [processId, setProcessId] = useState("");
  const [error, setError] = useState("");
  const [elapsed, setElapsed] = useState(0);
  const startedAt = useRef(0);
  const accumulated = useRef(0);
  const operation = useRef(false);

  useEffect(() => {
    if (status !== "recording") return;
    const timer = window.setInterval(
      () => setElapsed(accumulated.current + Date.now() - startedAt.current),
      250,
    );
    return () => window.clearInterval(timer);
  }, [status]);

  async function control(action: "start" | "pause" | "resume" | "stop") {
    if (operation.current) return;
    if (
      action === "start" &&
      (!Number.isInteger(Number(processId)) ||
        Number(processId) <= 0 ||
        Number(processId) > 4294967295)
    ) {
      setError("Informe um PID válido para o aplicativo da chamada.");
      return;
    }
    operation.current = true;
    setPending(true);
    setError("");
    onBusyChange(true);
    let nextStatus = status;
    try {
      if (action === "start") {
        await recordingService.start(Number(processId));
        accumulated.current = 0;
        startedAt.current = Date.now();
        setElapsed(0);
        nextStatus = "recording";
      } else if (action === "pause") {
        await recordingService.pause();
        accumulated.current += Date.now() - startedAt.current;
        setElapsed(accumulated.current);
        nextStatus = "paused";
      } else if (action === "resume") {
        await recordingService.resume();
        startedAt.current = Date.now();
        nextStatus = "recording";
      } else {
        const path = await recordingService.stop();
        if (status === "recording")
          accumulated.current += Date.now() - startedAt.current;
        setElapsed(accumulated.current);
        nextStatus = "finished";
        onFinished(path);
      }
    } catch (cause) {
      setError(errorMessage(cause));
      // stop_recording always releases the backend session, even on capture failure.
      if (action === "stop") nextStatus = "idle";
    } finally {
      setStatus(nextStatus);
      setPending(false);
      operation.current = false;
      onBusyChange(nextStatus === "recording" || nextStatus === "paused");
    }
  }

  const seconds = Math.floor(elapsed / 1000);
  const time = [
    Math.floor(seconds / 3600),
    Math.floor(seconds / 60) % 60,
    seconds % 60,
  ]
    .map((part) => String(part).padStart(2, "0"))
    .join(":");
  const labels = {
    idle: "Pronto para gravar",
    recording: "Gravando",
    paused: "Gravação pausada",
    finished: "Áudio finalizado",
  };
  const active = status === "recording" || status === "paused";
  return (
    <section className="recorder" aria-labelledby="recorder-title">
      <div className="section-heading">
        <div>
          <span className="eyebrow">02 / ÁUDIO</span>
          <h2 id="recorder-title">Grave o atendimento</h2>
        </div>
        <span className={`status ${status}`}>
          <i />
          {pending ? "Aguarde…" : labels[status]}
        </span>
      </div>
      <p className="muted">
        Capture o áudio do aplicativo da chamada. A gravação continua até você
        finalizar.
      </p>
      <label className="pid-field">
        PID do aplicativo
        <input
          type="number"
          min="1"
          max="4294967295"
          step="1"
          placeholder="Ex.: 12480"
          value={processId}
          disabled={active || pending || disabled || status === "finished"}
          onChange={(event) => setProcessId(event.target.value)}
        />
      </label>
      <p className="hint">
        Encontre o PID na aba Detalhes do Gerenciador de Tarefas do Windows. O
        gravador atual captura o aplicativo; o microfone ainda não está
        incluído.
      </p>
      <div className="recording-display">
        <span
          className={`record-dot ${status === "recording" ? "live" : ""}`}
        />
        <output aria-label="Tempo de gravação">{time}</output>
        <span className="format-tag">MP3 · 128 kbps</span>
      </div>
      <div className="button-row">
        {status === "idle" && (
          <button
            type="button"
            className="primary"
            disabled={pending || disabled}
            onClick={() => control("start")}
          >
            ● Iniciar gravação
          </button>
        )}
        {status === "recording" && (
          <button
            type="button"
            disabled={pending || disabled}
            onClick={() => control("pause")}
          >
            Ⅱ Pausar
          </button>
        )}
        {status === "paused" && (
          <button
            type="button"
            disabled={pending || disabled}
            onClick={() => control("resume")}
          >
            ▶ Retomar
          </button>
        )}
        {active && (
          <button
            type="button"
            className="stop"
            disabled={pending || disabled}
            onClick={() => control("stop")}
          >
            ■ Finalizar gravação
          </button>
        )}
        {status === "finished" && (
          <span className="success">
            ✓ Áudio salvo. Você já pode transcrever.
          </span>
        )}
      </div>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </section>
  );
}
