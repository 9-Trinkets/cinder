import * as api from '../api'

const CELL = 18
const PADDING = 10
const ROOM_SIZE = 8

export default function Minimap({
  map,
  uiText,
}: {
  map: api.MinimapData
  uiText: api.UiSnapshot['ui_text']
}) {
  const positions = new Map(map.rooms.map(room => [room.id, room]))
  const xs = map.rooms.map(room => room.x)
  const ys = map.rooms.map(room => room.y)
  const minX = Math.min(...xs)
  const maxX = Math.max(...xs)
  const minY = Math.min(...ys)
  const maxY = Math.max(...ys)
  const spanWidth = (maxX - minX) * CELL
  const spanHeight = (maxY - minY) * CELL
  const width = Math.max(150, spanWidth + PADDING * 2)
  const height = Math.max(96, spanHeight + PADDING * 2)
  const point = (x: number, y: number) => ({
    x: (x - minX) * CELL + (width - spanWidth) / 2,
    y: (y - minY) * CELL + (height - spanHeight) / 2,
  })

  return (
    <div className="space-y-2">
      <div className="relative overflow-hidden rounded border border-subtle bg-canvas/35 p-1.5">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 opacity-[0.08]"
          style={{
            backgroundImage: 'radial-gradient(circle, var(--color-muted) 0.6px, transparent 0.7px)',
            backgroundSize: '7px 7px',
          }}
        />
        <svg
          viewBox={`0 0 ${width} ${height}`}
          className="relative block h-auto max-h-52 w-full"
          role="img"
          aria-label={
            map.fully_revealed && map.total_count !== null
              ? `${map.label} minimap. ${map.visited_count} of ${map.total_count} rooms visited.`
              : `${map.label} minimap. ${map.visited_count} rooms charted.`
          }
        >
          <desc>
            {map.fully_revealed
              ? 'The complete floor is revealed.'
              : 'Only rooms you have visited are shown.'}
          </desc>
          <g
            fill="none"
            stroke="var(--color-muted)"
            strokeLinecap="round"
            strokeWidth="1.5"
            opacity="0.55"
          >
            {map.connections.map(connection => {
              const from = positions.get(connection.from)
              const to = positions.get(connection.to)
              if (!from || !to) return null
              const start = point(from.x, from.y)
              const end = point(to.x, to.y)
              return (
                <line
                  key={`${connection.from}:${connection.to}`}
                  x1={start.x}
                  y1={start.y}
                  x2={end.x}
                  y2={end.y}
                  vectorEffect="non-scaling-stroke"
                />
              )
            })}
          </g>
          {map.rooms.map(room => {
            const position = point(room.x, room.y)
            return (
              <g key={room.id}>
                <title>
                  {room.label}{room.current ? ' — current location' : room.visited ? ' — visited' : ' — revealed'}
                </title>
                {room.current && (
                  <circle
                    cx={position.x}
                    cy={position.y}
                    r={ROOM_SIZE * 0.9}
                    className="motion-safe:animate-pulse"
                    fill="none"
                    stroke="var(--color-gold)"
                    strokeWidth="1"
                    opacity="0.55"
                  />
                )}
                <rect
                  x={position.x - ROOM_SIZE / 2}
                  y={position.y - ROOM_SIZE / 2}
                  width={ROOM_SIZE}
                  height={ROOM_SIZE}
                  rx="1.5"
                  fill={
                    room.current
                      ? 'var(--color-gold)'
                      : room.visited
                        ? 'var(--color-pine)'
                        : 'var(--color-base)'
                  }
                  stroke={
                    room.current
                      ? 'var(--color-gold)'
                      : room.visited
                        ? 'var(--color-pine)'
                        : 'var(--color-muted)'
                  }
                  strokeWidth={room.current ? 2 : 1.25}
                  vectorEffect="non-scaling-stroke"
                />
                {room.current && (
                  <circle
                    cx={position.x}
                    cy={position.y}
                    r="1.5"
                    fill="var(--color-base)"
                  />
                )}
              </g>
            )
          })}
        </svg>
      </div>
      <div className="flex items-center justify-between gap-2 text-[10px] uppercase tracking-[0.12em] text-muted">
        <span>{map.label}</span>
        <span>
          {map.fully_revealed
            ? uiText.minimap_revealed_label
            : `${map.visited_count} ${uiText.minimap_charted_label}`}
        </span>
      </div>
    </div>
  )
}
