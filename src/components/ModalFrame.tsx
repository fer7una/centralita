import type { ReactNode } from 'react'

type ModalFrameProps = {
  ariaLabel: string
  children: ReactNode
  className?: string
  closeOnBackdropClick?: boolean
  closeLabel: string
  eyebrow: string
  onClose: () => void
  title: string
}

export function ModalFrame({
  ariaLabel,
  children,
  className,
  closeOnBackdropClick = true,
  closeLabel,
  eyebrow,
  onClose,
  title,
}: ModalFrameProps) {
  const modalClassName = className ? `card modal-card ${className}` : 'card modal-card'
  const handleBackdropClick = closeOnBackdropClick ? onClose : undefined

  return (
    <div className="modal-backdrop" onClick={handleBackdropClick} role="presentation">
      <section
        aria-label={ariaLabel}
        aria-modal="true"
        className={modalClassName}
        onClick={(event) => event.stopPropagation()}
        role="dialog"
      >
        <div className="section-title">
          <div>
            <p className="eyebrow">{eyebrow}</p>
            <h2>{title}</h2>
          </div>
          <button
            aria-label={closeLabel}
            className="modal-close"
            onClick={onClose}
            type="button"
          >
            x
          </button>
        </div>
        {children}
      </section>
    </div>
  )
}
