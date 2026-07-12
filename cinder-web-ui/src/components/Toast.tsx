import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from 'react'

interface Toast {
  id: number
  message: string
  kind: 'error' | 'success' | 'info'
}

interface ToastContextValue {
  showToast: (message: string, kind?: Toast['kind']) => void
}

const ToastContext = createContext<ToastContextValue>({ showToast: () => {} })

export function useToast() {
  return useContext(ToastContext)
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([])
  const [nextId, setNextId] = useState(0)

  const showToast = useCallback((message: string, kind: Toast['kind'] = 'info') => {
    setNextId(prev => prev + 1)
    setToasts(prev => [...prev, { id: nextId, message, kind }])
  }, [nextId])

  const dismiss = useCallback((id: number) => {
    setToasts(prev => prev.filter(t => t.id !== id))
  }, [])

  return (
    <ToastContext.Provider value={{ showToast }}>
      {children}
      <div className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
        {toasts.map(toast => (
          <ToastItem key={toast.id} toast={toast} onDismiss={() => dismiss(toast.id)} />
        ))}
      </div>
    </ToastContext.Provider>
  )
}

const kindClasses: Record<Toast['kind'], string> = {
  error: 'bg-love/20 border-love text-love',
  success: 'bg-pine/20 border-pine text-foam',
  info: 'bg-overlay border-subtle text-text',
}

function ToastItem({ toast, onDismiss }: { toast: Toast; onDismiss: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onDismiss, 5000)
    return () => clearTimeout(timer)
  }, [onDismiss])

  return (
    <div
      role="alert"
      className={`px-4 py-3 rounded border text-sm ${kindClasses[toast.kind]} shadow-lg`}
      onClick={onDismiss}
    >
      {toast.message}
    </div>
  )
}
