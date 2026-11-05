import { useEffect, useRef, useState } from "react";
import type Transcript from "../models/Transcript";
import type PageResponse from "../dtos/PageResponse";
import transcriptService from "../services/transcriptService";
import TranscriptTable from "../components/TranscriptTable";
import errorMessage from "../utils/errorMessage";

interface Props {
  onNew: () => void;
  onEdit: (item: Transcript) => void;
}
export default function TranscriptListPage({ onNew, onEdit }: Props) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState("");
  const [page, setPage] = useState(0);
  const [revision, setRevision] = useState(0);
  const [data, setData] = useState<PageResponse<Transcript>>({
    items: [],
    page: 0,
    size: 10,
    total: 0,
  });
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [deleting, setDeleting] = useState(false);
  const [selected, setSelected] = useState<Transcript | null>(null);
  const deleteLock = useRef(false);

  useEffect(() => {
    let current = true;
    setLoading(true);
    setError("");
    transcriptService
      .list(page, 10, filter)
      .then((result) => {
        if (!current) return;
        if (page > 0 && !result.items.length) {
          setPage(page - 1);
          return;
        }
        setData(result);
      })
      .catch((cause) => {
        if (current) setError(errorMessage(cause));
      })
      .finally(() => {
        if (current) setLoading(false);
      });
    return () => {
      current = false;
    };
  }, [page, filter, revision]);

  async function remove() {
    if (selected?.id == null || deleteLock.current) return;
    deleteLock.current = true;
    setDeleting(true);
    setError("");
    try {
      await transcriptService.remove(selected.id);
      setSelected(null);
      setRevision((value) => value + 1);
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setDeleting(false);
      deleteLock.current = false;
    }
  }

  return (
    <main className="page">
      <div className="page-heading">
        <div>
          <span className="eyebrow">SEU ESPAÇO DE ESCUTA</span>
          <h1>Atendimentos</h1>
          <p className="muted">Cada conversa, registrada em um só lugar.</p>
        </div>
        <button className="primary" onClick={onNew}>
          + Novo registro
        </button>
      </div>
      <section className="panel">
        <div className="list-toolbar">
          <div>
            <h2>Registros</h2>
            <span className="muted">
              {data.total} {data.total === 1 ? "atendimento" : "atendimentos"}
            </span>
          </div>
          <form
            className="search"
            onSubmit={(event) => {
              event.preventDefault();
              setPage(0);
              setFilter(query);
            }}
          >
            <label className="sr-only" htmlFor="search">
              Buscar por nome
            </label>
            <input
              id="search"
              placeholder="Buscar por nome…"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            <button type="submit">Buscar</button>
          </form>
        </div>
        {error && (
          <div className="error" role="alert">
            {error}{" "}
            <button
              className="text-button"
              onClick={() => setRevision((value) => value + 1)}
            >
              Tentar novamente
            </button>
          </div>
        )}
        {loading ? (
          <div className="empty" role="status">
            Carregando atendimentos…
          </div>
        ) : error ? null : data.items.length ? (
          <TranscriptTable
            items={data.items}
            onEdit={onEdit}
            onDelete={setSelected}
            disabled={deleting}
          />
        ) : (
          <div className="empty">
            <span className="empty-symbol">≋</span>
            <h2>
              {filter ? "Nenhum resultado" : "Sua próxima conversa começa aqui"}
            </h2>
            <p className="muted">
              {filter
                ? "Tente buscar por outro nome."
                : "Crie um registro, grave o atendimento e transforme o áudio em texto."}
            </p>
            {!filter && (
              <button onClick={onNew}>Criar primeiro registro</button>
            )}
          </div>
        )}
        <div className="pagination">
          <span>
            Página {page + 1} de {Math.max(1, Math.ceil(data.total / 10))}
          </span>
          <div className="button-row">
            <button
              disabled={loading || deleting || page === 0}
              onClick={() => setPage(page - 1)}
            >
              Anterior
            </button>
            <button
              disabled={loading || deleting || (page + 1) * 10 >= data.total}
              onClick={() => setPage(page + 1)}
            >
              Próxima
            </button>
          </div>
        </div>
      </section>
      {selected && (
        <div className="modal-backdrop">
          <section
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="delete-title"
          >
            <h2 id="delete-title">Excluir este registro?</h2>
            <p>
              O atendimento de <strong>{selected.name}</strong> será excluído.
              Esta ação não pode ser desfeita.
            </p>
            <div className="button-row">
              <button
                autoFocus
                disabled={deleting}
                onClick={() => setSelected(null)}
              >
                Cancelar
              </button>
              <button
                className="danger-button"
                disabled={deleting}
                onClick={remove}
              >
                {deleting ? "Excluindo…" : "Excluir registro"}
              </button>
            </div>
          </section>
        </div>
      )}
    </main>
  );
}
