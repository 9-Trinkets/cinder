CREATE TABLE IF NOT EXISTS bookmarks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    player_id UUID NOT NULL REFERENCES players(id) ON DELETE CASCADE,
    pack_id VARCHAR(255) NOT NULL,
    play_id UUID REFERENCES game_plays(id) ON DELETE SET NULL,
    label VARCHAR(255) NOT NULL DEFAULT '',
    locale VARCHAR(32) NOT NULL DEFAULT 'en',
    state_json JSONB NOT NULL,
    transcript_json JSONB NOT NULL DEFAULT '[]',
    turn_number INTEGER NOT NULL DEFAULT 0,
    current_room_id VARCHAR(255) NOT NULL DEFAULT '',
    current_room_name VARCHAR(255) NOT NULL DEFAULT '',
    day_number INTEGER NOT NULL DEFAULT 1,
    time_label VARCHAR(64) NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_bookmarks_player_pack
ON bookmarks(player_id, pack_id, created_at DESC);
