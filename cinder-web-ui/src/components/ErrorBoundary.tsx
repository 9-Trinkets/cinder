import { Component } from 'react'
import { useI18n } from '../i18n'

function ErrorFallback() {
  const { t } = useI18n()
  return <p className="text-love">{t('hud.errorReload')}</p>
}

export default class ErrorBoundary extends Component<{ children: React.ReactNode }, { error: Error | null }> {
  state = { error: null }
  static getDerivedStateFromError(error: Error) { return { error } }
  render() {
    if (this.state.error) {
      return (
        <div className="min-h-dvh flex items-center justify-center bg-surface text-text p-8">
          <ErrorFallback />
        </div>
      )
    }
    return this.props.children
  }
}
