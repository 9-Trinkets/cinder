import Modal from './Modal'
import Button from './Button'
import { useI18n } from '../i18n'

interface ConfirmDialogProps {
  title: string
  message: string
  confirmLabel?: string
  onConfirm: () => void
  onCancel: () => void
}

export default function ConfirmDialog({
  title,
  message,
  confirmLabel,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useI18n()
  return (
    <Modal title={title} onClose={onCancel}>
      <p className="text-sm text-text">{message}</p>
      <div className="flex gap-2 justify-end mt-4">
        <Button variant="ghost" onClick={onCancel}>{t('hud.cancel')}</Button>
        <Button variant="primary" onClick={onConfirm}>{confirmLabel ?? t('hud.confirm')}</Button>
      </div>
    </Modal>
  )
}
