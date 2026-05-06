import type { RuntimeStatus } from '../../types'

type RuntimeStatusBadgeProps = {
  status: RuntimeStatus
  variant?: 'default' | 'compact'
}

const labels: Record<RuntimeStatus, string> = {
  FAILED: 'FAILED',
  RUNNING: 'RUNNING',
  STARTING: 'STARTING',
  STOPPED: 'STOPPED',
  STOPPING: 'STOPPING',
}

export function RuntimeStatusBadge({
  status,
  variant = 'default',
}: RuntimeStatusBadgeProps) {
  const label = labels[status]
  const badgeClassName =
    variant === 'compact'
      ? `runtime-badge runtime-badge-compact status-${status.toLowerCase()}`
      : `runtime-badge status-${status.toLowerCase()}`

  return (
    <span aria-label={label} className={badgeClassName} title={label}>
      {variant === 'compact' ? <span className="sr-only">{label}</span> : label}
    </span>
  )
}
