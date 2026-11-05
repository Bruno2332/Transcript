import { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import type Transcript from "../models/Transcript";
import type TranscriptForm from "../models/TranscriptForm";
import transcriptService from "../services/transcriptService";
import errorMessage from "../utils/errorMessage";
import useCloseProtection from "../hooks/useCloseProtection";
import Recorder from "../components/Recorder";
import TranscriptionAction from "../components/TranscriptionAction";

interface Props {
  original?: Transcript;
  onBack: () => void;
  onSaved: () => void;
}
export default function TranscriptFormPage({
  original,
  onBack,
  onSaved,
}: Props) {
  const today = new Date();
  const localDate = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, "0")}-${String(today.getDate()).padStart(2, "0")}`;
  const {
    register,
    handleSubmit,
    getValues,
    setValue,
    formState: { errors, isDirty, isSubmitting },
  } = useForm<TranscriptForm>({
    defaultValues: {
      name: original?.name ?? "",
      dateOfService: original?.dateOfService ?? localDate,
      transcription: original?.transcription ?? "",
    },
  });
  const [audioPath, setAudioPath] = useState<string | null>(null);
  const [recording, setRecording] = useState(false);
  const [transcribing, setTranscribing] = useState(false);
  const [error, setError] = useState("");
  const [confirmLeave, setConfirmLeave] = useState(false);
  const busy = recording || transcribing || isSubmitting;
  useCloseProtection(isDirty || busy || !!audioPath, () =>
    setError(
      "Antes de fechar, finalize as operações e salve o registro ou volte à listagem para descartar as alterações.",
    ),
  );

  useEffect(() => {
    if (!isDirty && !busy && !audioPath) return;
    const prevent = (event: BeforeUnloadEvent) => {
      event.preventDefault();
      event.returnValue = "";
    };
    window.addEventListener("beforeunload", prevent);
    return () => window.removeEventListener("beforeunload", prevent);
  }, [isDirty, busy, audioPath]);

  async function save(values: TranscriptForm) {
    if (recording || transcribing) return;
    setError("");
    try {
      await transcriptService.save(values, original);
      onSaved();
    } catch (cause) {
      setError(errorMessage(cause));
    }
  }

  return (
    <main className="page form-page">
      <button
        className="back-button"
        disabled={busy}
        onClick={() =>
          isDirty || audioPath ? setConfirmLeave(true) : onBack()
        }
      >
        ← Voltar aos atendimentos
      </button>
      <div className="page-heading">
        <div>
          <span className="eyebrow">REGISTRO DE ATENDIMENTO</span>
          <h1>{original ? "Editar registro" : "Novo registro"}</h1>
          <p className="muted">Escute com atenção. Registre com clareza.</p>
        </div>
        <span className="draft-tag">
          {original ? "Registro salvo" : "Novo atendimento"}
        </span>
      </div>
      <form onSubmit={handleSubmit(save)} noValidate>
        <section className="panel form-section">
          <span className="eyebrow">01 / IDENTIFICAÇÃO</span>
          <h2>Dados do atendimento</h2>
          <div className="field-grid">
            <label>
              Nome da pessoa
              <input
                placeholder="Nome completo"
                disabled={isSubmitting}
                aria-invalid={!!errors.name}
                {...register("name", {
                  validate: (value) =>
                    !!value.trim() || "Informe o nome da pessoa.",
                })}
              />
              {errors.name && (
                <span className="field-error">{errors.name.message}</span>
              )}
            </label>
            <label>
              Data do atendimento
              <input
                type="date"
                disabled={isSubmitting}
                aria-invalid={!!errors.dateOfService}
                {...register("dateOfService", {
                  required: "Informe a data do atendimento.",
                  validate: (value) =>
                    /^\d{4}-\d{2}-\d{2}$/.test(value) ||
                    "Informe uma data válida.",
                })}
              />
              {errors.dateOfService && (
                <span className="field-error">
                  {errors.dateOfService.message}
                </span>
              )}
            </label>
          </div>
        </section>
        <div className="panel form-section">
          <Recorder
            disabled={transcribing || isSubmitting}
            onBusyChange={setRecording}
            onFinished={setAudioPath}
          />
          {audioPath && (
            <p className="audio-path">
              Arquivo salvo: <span>{audioPath}</span>
            </p>
          )}
        </div>
        <section className="panel form-section">
          <div className="section-heading">
            <div>
              <span className="eyebrow">03 / TRANSCRIÇÃO</span>
              <h2>O atendimento, em palavras</h2>
            </div>
          </div>
          <TranscriptionAction
            audioPath={audioPath}
            disabled={recording || isSubmitting}
            onBusyChange={setTranscribing}
            onComplete={(text) => {
              const existing = getValues("transcription");
              setValue(
                "transcription",
                existing.trim() ? `${existing}\n\n${text}` : text,
                { shouldDirty: true, shouldValidate: true },
              );
            }}
          />
          <label className="transcript-label">
            Transcrição
            <textarea
              rows={12}
              placeholder="O texto transcrito aparecerá aqui. Você também pode escrever ou editar suas anotações."
              disabled={transcribing || isSubmitting}
              {...register("transcription")}
            />
          </label>
          <p className="hint">
            Revise o texto e a identificação das pessoas antes de salvar.
          </p>
        </section>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <footer className="form-footer">
          <span className="muted">
            {recording
              ? "Finalize a gravação para salvar."
              : transcribing
                ? "Aguarde a transcrição para salvar."
                : "Os dados ficam salvos neste computador."}
          </span>
          <button type="submit" className="primary" disabled={busy}>
            {isSubmitting ? "Salvando…" : "Salvar registro"}
          </button>
        </footer>
      </form>
      {confirmLeave && (
        <div className="modal-backdrop">
          <section
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="leave-title"
          >
            <h2 id="leave-title">Sair sem salvar?</h2>
            <p>
              As alterações do formulário serão descartadas. O arquivo de áudio
              já finalizado permanece no computador.
            </p>
            <div className="button-row">
              <button autoFocus onClick={() => setConfirmLeave(false)}>
                Continuar editando
              </button>
              <button className="danger-button" onClick={onBack}>
                Sair sem salvar
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}
