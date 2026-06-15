-- Create players table
CREATE TABLE IF NOT EXISTS players (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Create game sessions table (stores serialized world state)
CREATE TABLE IF NOT EXISTS game_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    player_id TEXT NOT NULL REFERENCES players(id),
    pack_id TEXT NOT NULL,
    state_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_game_sessions_player_id ON game_sessions(player_id);
