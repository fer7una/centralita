import type { RuntimeLogLine } from '../../types'

type ProjectLogsPanelProps = {
  lines: RuntimeLogLine[]
}

export function ProjectLogsPanel({ lines }: ProjectLogsPanelProps) {
  if (lines.length === 0) {
    return (
      <div aria-label="Logs de terminal" className="log-console" role="log">
        <span>Todavía no hay logs.</span>
      </div>
    )
  }

  return (
    <div aria-label="Logs de terminal" className="log-console" role="log">
      {lines.map((line, index) => (
        <div
          data-stream={line.stream}
          key={`${line.timestamp}-${line.stream}-${index}`}
        >
          <span>{line.line}</span>
        </div>
      ))}
    </div>
  )
}
