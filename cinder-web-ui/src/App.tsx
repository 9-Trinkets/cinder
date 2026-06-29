import { Routes, Route, Navigate } from 'react-router-dom'
import { AuthProvider, useAuth } from './auth'
import LoginPage from './pages/LoginPage'
import GamesPage from './pages/GamesPage'
import GamePage from './pages/GamePage'

function ProtectedRoute({ children }: { children: React.ReactNode }) {
  const { token } = useAuth()
  if (!token) return <Navigate to="/login" replace />
  return <>{children}</>
}

function RedirectIfLoggedIn() {
  const { token } = useAuth()
  if (token) return <Navigate to="/games" replace />
  return <LoginPage />
}

export default function App() {
  return (
    <AuthProvider>
      <Routes>
        <Route path="/login" element={<RedirectIfLoggedIn />} />
        <Route path="/games" element={<ProtectedRoute><GamesPage /></ProtectedRoute>} />
        <Route path="/games/:id" element={<ProtectedRoute><GamePage /></ProtectedRoute>} />
        <Route path="*" element={<Navigate to="/login" replace />} />
      </Routes>
    </AuthProvider>
  )
}
