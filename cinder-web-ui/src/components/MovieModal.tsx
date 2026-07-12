import { useEffect } from 'react'
import * as api from '../api'

export default function MovieModal({ movie, frame, onAdvance, onClose }: {
  movie: api.MovieData
  frame: number
  onAdvance: () => void
  onClose: () => void
}) {
  const f = movie.frames[frame]
  const isLast = frame >= movie.frames.length - 1

  useEffect(() => {
    if (!f || isLast) return
    const timer = setTimeout(onAdvance, Math.max(300, f.duration_ms))
    return () => clearTimeout(timer)
  }, [frame, f, isLast, onAdvance])

  if (!f) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black" onClick={onClose}>
      <div
        className="max-w-2xl w-full mx-4"
        onClick={e => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-base font-semibold text-text">{movie.title}</h2>
          <div className="flex items-center gap-2">
            <span className="text-xs text-muted">{frame + 1} / {movie.frames.length}</span>
            <button onClick={onClose} className="text-muted hover:text-text text-lg leading-none cursor-pointer">&times;</button>
          </div>
        </div>
        <pre className="text-pine text-xs leading-none whitespace-pre-wrap font-mono bg-black/40 rounded-lg p-4 max-h-[60vh] overflow-y-auto border border-subtle select-none">
          {f.text}
        </pre>
        {isLast && (
          <p className="text-center text-muted text-sm mt-3 animate-pulse cursor-pointer" onClick={onClose}>Click or tap to continue...</p>
        )}
      </div>
    </div>
  )
}
