import type { ButtonHTMLAttributes } from 'react'

type BlockedActionButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  blockedReason?: string
}

export function BlockedActionButton({
  blockedReason,
  children,
  disabled,
  title,
  ...buttonProps
}: BlockedActionButtonProps) {
  const shouldExposeBlockedReason = Boolean(disabled && blockedReason)
  const button = (
    <button
      {...buttonProps}
      disabled={disabled}
      title={shouldExposeBlockedReason ? undefined : title}
    >
      {children}
    </button>
  )

  if (!shouldExposeBlockedReason) {
    return button
  }

  return (
    <span
      aria-label={blockedReason}
      className="blocked-action-tooltip"
      title={blockedReason}
    >
      {button}
    </span>
  )
}
