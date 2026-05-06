import type { HealthStatus } from '../../types'

type HealthStatusBadgeProps = {
  status: HealthStatus
  variant?: 'default' | 'compact'
}

const labels: Record<HealthStatus, string> = {
  CHECKING: 'CHECKING',
  HEALTHY: 'HEALTHY',
  UNKNOWN: 'UNKNOWN',
  UNHEALTHY: 'UNHEALTHY',
  UNSUPPORTED: 'UNSUPPORTED',
}

export function HealthStatusBadge({
  status,
  variant = 'default',
}: HealthStatusBadgeProps) {
  const label = labels[status]
  const badgeClassName =
    variant === 'compact'
      ? `health-badge health-badge-compact health-${status.toLowerCase()}`
      : `health-badge health-${status.toLowerCase()}`

  return (
    <span aria-label={label} className={badgeClassName} title={label}>
      {variant === 'compact' ? <span className="sr-only">{label}</span> : label}
    </span>
  )
}
