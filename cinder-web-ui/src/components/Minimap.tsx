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
                {/* Portal aura */}
                {room.has_teleport && (
                  <circle
                    cx={position.x}
                    cy={position.y}
                    r={ROOM_SIZE * 0.95}
                    fill="none"
                    stroke="var(--color-foam)"
                    strokeWidth="1.2"
                    strokeDasharray="2.5 1.5"
                    className="motion-safe:animate-spin"
                    style={{
                      transformOrigin: `${position.x}px ${position.y}px`,
                      animationDuration: '10s',
                      opacity: 0.85,
                    }}
                  />
                )}

                {/* Player current pulse */}
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

                {/* Room node rect */}
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

                {/* Center marker: current player dot or portal diamond */}
                {room.current ? (
                  <circle
                    cx={position.x}
                    cy={position.y}
                    r="1.5"
                    fill="var(--color-base)"
                  />
                ) : room.has_teleport ? (
                  <polygon
                    points={`
                      ${position.x},${position.y - 2.5}
                      ${position.x + 2.5},${position.y}
                      ${position.x},${position.y + 2.5}
                      ${position.x - 2.5},${position.y}
                    `}
                    fill="var(--color-foam)"
                    opacity="0.95"
                  />
                ) : null}

                {/* Live Ally Tracker (top-left badge) */}
                {Boolean(room.ally_count && room.ally_count > 0) && (
                  <g>
                    <circle
                      cx={position.x - ROOM_SIZE / 2}
                      cy={position.y - ROOM_SIZE / 2}
                      r="2.2"
                      fill="var(--color-pine)"
                      stroke="var(--color-base)"
                      strokeWidth="0.75"
                    />
                    <circle
                      cx={position.x - ROOM_SIZE / 2}
                      cy={position.y - ROOM_SIZE / 2}
                      r="3.2"
                      fill="none"
                      stroke="var(--color-pine)"
                      strokeWidth="0.5"
                      opacity="0.6"
                      className="motion-safe:animate-ping"
                      style={{
                        transformOrigin: `${position.x - ROOM_SIZE / 2}px ${position.y - ROOM_SIZE / 2}px`,
                        animationDuration: '3s',
                      }}
                    />
                  </g>
                )}

                {/* Live Hostile Tracker (top-right badge) */}
                {Boolean(room.hostile_count && room.hostile_count > 0) && (
                  <g>
                    <circle
                      cx={position.x + ROOM_SIZE / 2}
                      cy={position.y - ROOM_SIZE / 2}
                      r="2.2"
                      fill="var(--color-love)"
                      stroke="var(--color-base)"
                      strokeWidth="0.75"
                    />
                    <circle
                      cx={position.x + ROOM_SIZE / 2}
                      cy={position.y - ROOM_SIZE / 2}
                      r="3.2"
                      fill="none"
                      stroke="var(--color-love)"
                      strokeWidth="0.5"
                      opacity="0.6"
                      className="motion-safe:animate-ping"
                      style={{
                        transformOrigin: `${position.x + ROOM_SIZE / 2}px ${position.y - ROOM_SIZE / 2}px`,
                        animationDuration: '2s',
                      }}
                    />
                  </g>
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
      {(map.has_teleports || map.entity_tracking) && (
        <div className="flex flex-wrap items-center gap-3 pt-1 border-t border-subtle/50 text-[10px] tracking-wider text-muted">
          {map.has_teleports && (
            <span className="inline-flex items-center gap-1.5">
              <span className="inline-block h-2 w-2 rounded-full border border-dashed border-[var(--color-foam)] bg-[var(--color-foam)]/30" />
              <span>Portal</span>
            </span>
          )}
          {map.entity_tracking && (
            <>
              <span className="inline-flex items-center gap-1.5">
                <span className="inline-block h-2 w-2 rounded-full bg-[var(--color-pine)]" />
                <span>Allies</span>
              </span>
              <span className="inline-flex items-center gap-1.5">
                <span className="inline-block h-2 w-2 rounded-full bg-[var(--color-love)]" />
                <span>Hostiles</span>
              </span>
            </>
          )}
        </div>
      )}
    </div>
  )
}
