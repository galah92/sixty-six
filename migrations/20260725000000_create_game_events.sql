CREATE TABLE IF NOT EXISTS game_events (
    room_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    deal_number INTEGER NOT NULL,
    actor TEXT NOT NULL CHECK (actor IN ('one', 'two', 'system')),
    event_type TEXT NOT NULL,
    action_json TEXT,
    state_before_json TEXT,
    state_after_json TEXT,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    PRIMARY KEY (room_id, revision, event_type)
);

CREATE INDEX IF NOT EXISTS game_events_room_revision_idx
    ON game_events (room_id, revision);

CREATE INDEX IF NOT EXISTS game_events_expires_at_idx
    ON game_events (expires_at);

DELETE FROM game_events
WHERE event_type = 'legacy_action'
  AND EXISTS (
      SELECT 1
      FROM game_events AS current_event
      WHERE current_event.room_id = game_events.room_id
        AND current_event.revision = game_events.revision
        AND current_event.event_type = 'action'
  );

INSERT OR IGNORE INTO game_events (
    room_id,
    revision,
    deal_number,
    actor,
    event_type,
    action_json,
    state_before_json,
    state_after_json,
    created_at,
    expires_at
)
SELECT
    room_id,
    revision,
    0,
    actor,
    'legacy_action',
    action_json,
    NULL,
    NULL,
    created_at,
    created_at + (90 * 24 * 60 * 60)
FROM actions
WHERE NOT EXISTS (
    SELECT 1
    FROM game_events
    WHERE game_events.room_id = actions.room_id
      AND game_events.revision = actions.revision
);
