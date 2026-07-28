-- Rename game_sessions → game_plays
ALTER TABLE transcript_entries DROP CONSTRAINT IF EXISTS transcript_entries_session_id_fkey;
ALTER TABLE checkpoints DROP CONSTRAINT IF EXISTS checkpoints_session_id_fkey;

ALTER TABLE IF EXISTS game_sessions RENAME TO game_plays;
ALTER INDEX IF EXISTS idx_game_sessions_player_id RENAME TO idx_game_plays_player_id;

ALTER TABLE IF EXISTS transcript_entries RENAME COLUMN session_id TO play_id;
ALTER TABLE IF EXISTS checkpoints RENAME COLUMN session_id TO play_id;

ALTER TABLE transcript_entries
    ADD CONSTRAINT transcript_entries_play_id_fkey
    FOREIGN KEY (play_id) REFERENCES game_plays(id) ON DELETE CASCADE;

ALTER TABLE checkpoints
    ADD CONSTRAINT checkpoints_play_id_fkey
    FOREIGN KEY (play_id) REFERENCES game_plays(id) ON DELETE CASCADE;
