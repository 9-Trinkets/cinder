import { memo } from 'react'

export interface Line {
  text: string
  key: number
}

const TranscriptLine = memo(function TranscriptLine({ line }: { line: Line }) {
  return (
    <div className="whitespace-pre-wrap text-sm leading-relaxed">
      {line.text.startsWith('> ') ? (
        <span className="text-foam">{line.text}</span>
      ) : line.text.startsWith('== ') ? (
        <span className="text-iris font-bold">{line.text}</span>
      ) : (
        <span className="text-text">{line.text}</span>
      )}
    </div>
  )
})

export default TranscriptLine
