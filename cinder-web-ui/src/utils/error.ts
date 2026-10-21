import { LOCALES, resolveLocale, type MessageKey } from '../i18n/strings'

/** Raw English messages the server sends, mapped to localized UI strings. */
const SERVER_MESSAGES: [RegExp, MessageKey][] = [
  [/^invalid credentials$/i, 'error.invalidCredentials'],
  [/^username taken$/i, 'error.usernameTaken'],
  [/^username must be/i, 'error.usernameLength'],
  [/^username: letters/i, 'error.usernameChars'],
  [/^password must be/i, 'error.passwordLength'],
  [/^A running chronicle already exists/i, 'error.runningPlayExists'],
  [
    /^(invalid or expired token|not authenticated|missing Authorization header|invalid Authorization format|invalid token subject)$/i,
    'error.sessionExpired',
  ],
  [/^(play|bookmark|player) not found$/i, 'error.notFound'],
  [/^play changed too frequently/i, 'error.retry'],
  [/^cannot bookmark when game is over$/i, 'error.gameOver'],
  [/^Backend unreachable$/i, 'error.unreachable'],
]

function currentLocale(): string {
  try {
    return resolveLocale(localStorage.getItem('uiLocale'))
  } catch {
    return resolveLocale(null)
  }
}

export function toErrorMessage(err: unknown, fallback: string): string {
  if (!(err instanceof Error) || !err.message) return fallback
  const hit = SERVER_MESSAGES.find(([pattern]) => pattern.test(err.message.trim()))
  if (!hit) return err.message
  return (LOCALES[currentLocale()] ?? LOCALES.en)[hit[1]]
}
