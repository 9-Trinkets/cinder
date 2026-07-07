ALTER TABLE game_sessions
ADD COLUMN IF NOT EXISTS locale VARCHAR(32) NOT NULL DEFAULT 'en';

CREATE TABLE IF NOT EXISTS checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
    locale VARCHAR(32) NOT NULL,
    state_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_checkpoints_session_created_at
ON checkpoints(session_id, created_at DESC);
