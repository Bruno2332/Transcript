import type Transcript from "../models/Transcript";

interface Props {
  items: Transcript[];
  onEdit: (item: Transcript) => void;
  onDelete: (item: Transcript) => void;
  disabled: boolean;
}

export default function TranscriptTable({
  items,
  onEdit,
  onDelete,
  disabled,
}: Props) {
  return (
    <div className="table-wrap">
      <table>
        <thead>
          <tr>
            <th>Nome da pessoa</th>
            <th>Data do atendimento</th>
            <th>
              <span className="sr-only">Ações</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {items.map((item) => (
            <tr key={item.id}>
              <td>
                <button
                  className="name-button"
                  onClick={() => onEdit(item)}
                  disabled={disabled}
                >
                  <span className="avatar" aria-hidden="true">
                    {item.name.slice(0, 1).toUpperCase()}
                  </span>
                  {item.name}
                </button>
              </td>
              <td>{item.dateOfService.split("-").reverse().join("/")}</td>
              <td className="row-actions">
                <button
                  className="text-button"
                  disabled={disabled}
                  onClick={() => onEdit(item)}
                >
                  Abrir
                </button>
                <button
                  className="text-button danger"
                  disabled={disabled}
                  onClick={() => onDelete(item)}
                  aria-label={`Excluir registro de ${item.name}`}
                >
                  Excluir
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
