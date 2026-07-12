import { memo } from 'react'
import * as api from '../api'

const SessionClosureModal = memo(function SessionClosureModal({
  sessionClosure,
  onDismiss,
}: {
  sessionClosure: api.SessionClosureData
  onDismiss: () => void
}) {
  return (
    <div className="fixed inset-0 bg-base/80 flex items-center justify-center z-50">
      <div className="bg-surface rounded-xl p-6 max-w-md w-full mx-4 shadow-2xl">
        <h2 className="text-xl font-bold text-center mb-2">{sessionClosure.title}</h2>
        {sessionClosure.subtitle && (
          <p className="text-center text-sm text-muted mb-4">— {sessionClosure.subtitle}</p>
        )}
        <div className="space-y-4">
          {sessionClosure.sections.map((section, index) => (
            <div key={`${section.kind}-${index}`}>
              {section.title && (
                <p className="text-xs text-muted uppercase tracking-wider mb-2">{section.title}</p>
              )}
              {section.kind === 'rating' ? (
                <div className="flex justify-center gap-1 text-2xl">
                  {Array.from({ length: section.max }, (_, i) => i + 1).map(n => (
                    <span key={n} className={n <= section.value ? 'text-yellow-400' : 'text-muted'}>
                      {n <= section.value ? '\u2605' : '\u2606'}
                    </span>
                  ))}
                </div>
              ) : (
                <p className="text-center text-balance leading-relaxed whitespace-pre-wrap">{section.body}</p>
              )}
            </div>
          ))}
        </div>
        <button
          className="mt-6 w-full py-2 bg-love text-white rounded-lg font-semibold hover:opacity-90 cursor-pointer"
          onClick={onDismiss}
        >
          OK
        </button>
      </div>
    </div>
  )
})

export default SessionClosureModal
