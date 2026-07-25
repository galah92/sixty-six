CREATE TABLE IF NOT EXISTS rooms (
    id TEXT PRIMARY KEY NOT NULL,
    mode TEXT NOT NULL CHECK (mode IN ('computer', 'friend')),
    status TEXT NOT NULL CHECK (status IN ('waiting', 'active', 'finished')),
    revision INTEGER NOT NULL DEFAULT 0,
    state_json TEXT NOT NULL,
    player_one_name TEXT NOT NULL,
    player_one_token_hash TEXT NOT NULL,
    player_two_name TEXT,
    player_two_token_hash TEXT,
    player_one_seen_at INTEGER NOT NULL,
    player_two_seen_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS rooms_expires_at_idx ON rooms (expires_at);

CREATE TABLE IF NOT EXISTS actions (
    room_id TEXT NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    revision INTEGER NOT NULL,
    actor TEXT NOT NULL CHECK (actor IN ('one', 'two', 'system')),
    action_json TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (room_id, revision)
);
