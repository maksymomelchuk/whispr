import { useState, type ReactNode } from 'react'

interface Props {
  title: string
  defaultOpen?: boolean
  dirty?: boolean
  children: ReactNode
}

export function CollapsibleCard({
  title,
  defaultOpen = true,
  dirty = false,
  children,
}: Props) {
  const [open, setOpen] = useState(defaultOpen)

  return (
    <section className={`card collapsible-card ${open ? 'open' : 'closed'}`}>
      <button
        type="button"
        className="collapsible-header"
        aria-expanded={open}
        onClick={() => setOpen((v) => !v)}
      >
        <svg
          className={`chevron ${open ? 'open' : ''}`}
          viewBox="0 0 12 12"
          width="10"
          height="10"
          aria-hidden="true"
        >
          <path
            d="M4 2.5l4 3.5-4 3.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        <h2>{title}</h2>
        {dirty && !open && (
          <span className="dirty-dot" aria-label="Unsaved changes" />
        )}
      </button>
      <div className="collapsible-wrap" data-open={open}>
        <div className="collapsible-body" aria-hidden={!open}>
          {children}
        </div>
      </div>
    </section>
  )
}
