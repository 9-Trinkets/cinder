import { Component } from 'react'

export default class ErrorBoundary extends Component<{ children: React.ReactNode }, { error: Error | null }> {
  state = { error: null }
  static getDerivedStateFromError(error: Error) { return { error } }
  render() {
    if (this.state.error) {
      return (
        <div className="min-h-dvh flex items-center justify-center bg-surface text-text p-8">
          <p className="text-love">Something went wrong. Please reload the page.</p>
        </div>
      )
    }
    return this.props.children
  }
}
