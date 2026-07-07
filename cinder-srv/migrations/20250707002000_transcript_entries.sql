CREATE TABLE IF NOT EXISTS transcript_entries (
    id BIGSERIAL PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES game_sessions(id) ON DELETE CASCADE,
    turn_number INTEGER NOT NULL,
    role VARCHAR(32) NOT NULL,
    text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_transcript_entries_session
ON transcript_entries(session_id, turn_number ASC, id ASC);
