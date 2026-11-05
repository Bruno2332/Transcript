import { useRef, useState } from "react";
import transcriptionService from "../services/transcriptionService";
import errorMessage from "../utils/errorMessage";

interface Props {
  audioPath: string | null;
  disabled: boolean;
  onBusyChange: (busy: boolean) => void;
  onComplete: (text: string) => void;
}
export default function TranscriptionAction({
  audioPath,
  disabled,
  onBusyChange,
  onComplete,
}: Props) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const operation = useRef(false);
  async function transcribe() {
    if (!audioPath || operation.current) return;
    operation.current = true;
    setBusy(true);
    setError("");
    onBusyChange(true);
    try {
      const text = await transcriptionService.transcribe(audioPath);
      if (!text.trim()) setError("Nenhuma fala foi identificada no áudio.");
      else onComplete(text);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusy(false);
      onBusyChange(false);
      operation.current = false;
    }
  }
  return (
    <div>
      <button
        type="button"
        className="secondary"
        disabled={!audioPath || disabled || busy}
        onClick={transcribe}
      >
        {busy ? "Transcrevendo áudio…" : "✦ Transcrever áudio"}
      </button>
      <p className="hint">
        {busy
          ? "Aguarde nesta tela. Áudios longos podem levar vários minutos."
          : "O áudio será enviado à Deepgram. O texto será adicionado ao campo abaixo."}
      </p>
      {error && (
        <p role="alert" className="error">
          {error}
        </p>
      )}
    </div>
  );
}
