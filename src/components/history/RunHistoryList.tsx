import type { RunHistoryEntry } from '../../types'

type RunHistoryListProps = {
  entries: RunHistoryEntry[]
}

function formatDate(value: string | null | undefined) {
  if (!value) {
    return 'Sin dato'
  }

  return new Date(value).toLocaleString()
}

export function RunHistoryList({ entries }: RunHistoryListProps) {
  if (entries.length === 0) {
    return <p className="muted">Todavía no hay ejecuciones registradas para este proyecto.</p>
  }

  return (
    <ul className="detail-list">
      {entries.map((entry) => (
        <li key={entry.id}>
          <strong>{entry.commandPreview}</strong>
          <span>
            Inicio: {formatDate(entry.startedAt)} · Fin: {formatDate(entry.endedAt)}
          </span>
          <span>
            Runtime final: {entry.finalRuntimeStatus}
            {entry.finalHealthStatus ? ` · Health: ${entry.finalHealthStatus}` : ''}
            {entry.exitCode !== null && entry.exitCode !== undefined ? ` · Exit: ${entry.exitCode}` : ''}
          </span>
          <span>
            {entry.stopReason ?? 'Sin motivo registrado'}
            {entry.errorMessage ? ` · ${entry.errorMessage}` : ''}
          </span>
        </li>
      ))}
    </ul>
  )
}
