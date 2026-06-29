import { createContext, useContext, useState, useCallback, type ReactNode } from 'react'
import * as api from './api'

interface AuthContextValue {
  token: string | null
  playerId: string | null
  login: (username: string, password: string) => Promise<void>
  signup: (username: string, password: string) => Promise<void>
  logout: () => void
}

const AuthContext = createContext<AuthContextValue | null>(null)

export function AuthProvider({ children }: { children: ReactNode }) {
  const [token, setToken] = useState<string | null>(() => localStorage.getItem('token'))
  const [playerId, setPlayerId] = useState<string | null>(() => localStorage.getItem('playerId'))

  const login = useCallback(async (username: string, password: string) => {
    const res = await api.login(username, password)
    localStorage.setItem('token', res.token)
    localStorage.setItem('playerId', res.player_id)
    setToken(res.token)
    setPlayerId(res.player_id)
  }, [])

  const signup = useCallback(async (username: string, password: string) => {
    const res = await api.signup(username, password)
    localStorage.setItem('token', res.token)
    localStorage.setItem('playerId', res.player_id)
    setToken(res.token)
    setPlayerId(res.player_id)
  }, [])

  const logout = useCallback(() => {
    localStorage.removeItem('token')
    localStorage.removeItem('playerId')
    setToken(null)
    setPlayerId(null)
  }, [])

  return (
    <AuthContext.Provider value={{ token, playerId, login, signup, logout }}>
      {children}
    </AuthContext.Provider>
  )
}

export function useAuth(): AuthContextValue {
  const ctx = useContext(AuthContext)
  if (!ctx) throw new Error('useAuth must be used within AuthProvider')
  return ctx
}
