use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::{Row, SqlitePool};
use thiserror::Error;

use crate::game::{Action, MatchState, Seat};

const ACTIVE_ROOM_TTL_SECONDS: i64 = 7 * 24 * 60 * 60;
const WAITING_ROOM_TTL_SECONDS: i64 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomMode {
    Computer,
    Friend,
}

impl RoomMode {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Computer => "computer",
            Self::Friend => "friend",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewRoom<'a> {
    pub id: &'a str,
    pub mode: RoomMode,
    pub state: &'a MatchState,
    pub player_one_name: &'a str,
    pub player_one_token: &'a str,
    pub player_two_name: Option<&'a str>,
    pub player_two_token: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct Room {
    pub id: String,
    pub mode: RoomMode,
    pub status: String,
    pub revision: i64,
    pub state: MatchState,
    pub player_names: [Option<String>; 2],
    pub player_token_hashes: [Option<String>; 2],
    pub last_seen_at: [Option<i64>; 2],
    pub updated_at: i64,
}

impl Room {
    #[must_use]
    pub fn seat_for_token(&self, token: &str) -> Option<Seat> {
        let hash = hash_token(token);
        if self.player_token_hashes[0].as_deref() == Some(hash.as_str()) {
            Some(Seat::One)
        } else if self.player_token_hashes[1].as_deref() == Some(hash.as_str()) {
            Some(Seat::Two)
        } else {
            None
        }
    }

    #[must_use]
    pub fn has_second_player(&self) -> bool {
        self.player_names[1].is_some()
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("room not found")]
    NotFound,
    #[error("room is already full")]
    RoomFull,
    #[error("the game changed; refresh and try again")]
    Conflict,
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("stored game state is invalid: {0}")]
    InvalidState(#[from] serde_json::Error),
    #[error("game rule rejected the action: {0}")]
    Rule(#[from] crate::game::RuleError),
}

#[derive(Clone)]
pub struct Store {
    pool: SqlitePool,
}

impl Store {
    /// Opens the `SQLite` pool and installs the current schema.
    ///
    /// # Errors
    ///
    /// Returns a database error when the URL is invalid, `SQLite` cannot be
    /// opened, or the schema cannot be installed.
    pub async fn connect(database_url: &str) -> Result<Self, StoreError> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(options)
            .await?;
        sqlx::raw_sql(include_str!(
            "../migrations/20260724000000_create_rooms.sql"
        ))
        .execute(&pool)
        .await?;
        Ok(Self { pool })
    }

    /// Persists a newly created room and its initial player seats.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization or the atomic insert fails.
    pub async fn create_room(&self, room: NewRoom<'_>) -> Result<(), StoreError> {
        let now = now_epoch();
        let status = if room.mode == RoomMode::Friend && room.player_two_name.is_none() {
            "waiting"
        } else {
            "active"
        };
        sqlx::query(
            r"
            INSERT INTO rooms (
                id, mode, status, revision, state_json,
                player_one_name, player_one_token_hash,
                player_two_name, player_two_token_hash,
                player_one_seen_at, player_two_seen_at,
                created_at, updated_at, expires_at
            ) VALUES (?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ",
        )
        .bind(room.id)
        .bind(room.mode.as_str())
        .bind(status)
        .bind(serde_json::to_string(room.state)?)
        .bind(room.player_one_name)
        .bind(hash_token(room.player_one_token))
        .bind(room.player_two_name)
        .bind(room.player_two_token.map(hash_token))
        .bind(now)
        .bind(room.player_two_name.map(|_| now))
        .bind(now)
        .bind(now)
        .bind(
            now + if status == "waiting" {
                WAITING_ROOM_TTL_SECONDS
            } else {
                ACTIVE_ROOM_TTL_SECONDS
            },
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Loads one unexpired room.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::NotFound`] for a missing or expired room, or a
    /// storage error when the row cannot be decoded.
    pub async fn load_room(&self, room_id: &str) -> Result<Room, StoreError> {
        let row = sqlx::query(
            r"
            SELECT id, mode, status, revision, state_json,
                   player_one_name, player_one_token_hash,
                   player_two_name, player_two_token_hash,
                   player_one_seen_at, player_two_seen_at, updated_at
            FROM rooms
            WHERE id = ? AND expires_at > ?
            ",
        )
        .bind(room_id)
        .bind(now_epoch())
        .fetch_optional(&self.pool)
        .await?
        .ok_or(StoreError::NotFound)?;
        row_to_room(&row)
    }

    /// Atomically claims the second seat in a private friend room.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::RoomFull`] when the seat is already occupied,
    /// [`StoreError::NotFound`] for a missing room, or a database error.
    pub async fn join_friend(
        &self,
        room_id: &str,
        name: &str,
        token: &str,
    ) -> Result<Room, StoreError> {
        let now = now_epoch();
        let result = sqlx::query(
            r"
            UPDATE rooms
            SET player_two_name = ?, player_two_token_hash = ?,
                player_two_seen_at = ?, status = 'active',
                revision = revision + 1, updated_at = ?, expires_at = ?
            WHERE id = ? AND mode = 'friend' AND player_two_name IS NULL AND expires_at > ?
            ",
        )
        .bind(name)
        .bind(hash_token(token))
        .bind(now)
        .bind(now)
        .bind(now + ACTIVE_ROOM_TTL_SECONDS)
        .bind(room_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            let room = self.load_room(room_id).await?;
            return if room.has_second_player() {
                Err(StoreError::RoomFull)
            } else {
                Err(StoreError::Conflict)
            };
        }
        self.load_room(room_id).await
    }

    /// Applies one revision-checked game action and appends it to the audit log.
    ///
    /// # Errors
    ///
    /// Returns [`StoreError::Conflict`] for a stale revision, a rule error for
    /// an illegal action, or a persistence/serialization error.
    pub async fn apply_action(
        &self,
        room_id: &str,
        expected_revision: i64,
        actor: Seat,
        action: Action,
    ) -> Result<Room, StoreError> {
        let mut transaction = self.pool.begin().await?;
        let row =
            sqlx::query("SELECT revision, state_json FROM rooms WHERE id = ? AND expires_at > ?")
                .bind(room_id)
                .bind(now_epoch())
                .fetch_optional(&mut *transaction)
                .await?
                .ok_or(StoreError::NotFound)?;
        let revision: i64 = row.try_get("revision")?;
        if revision != expected_revision {
            return Err(StoreError::Conflict);
        }
        let state_json: String = row.try_get("state_json")?;
        let mut state: MatchState = serde_json::from_str(&state_json)?;
        state.apply(actor, action)?;
        let next_revision = revision + 1;
        let now = now_epoch();
        let status = if state.winner.is_some() {
            "finished"
        } else {
            "active"
        };
        let updated = sqlx::query(
            r"
            UPDATE rooms
            SET revision = ?, state_json = ?, status = ?, updated_at = ?, expires_at = ?
            WHERE id = ? AND revision = ?
            ",
        )
        .bind(next_revision)
        .bind(serde_json::to_string(&state)?)
        .bind(status)
        .bind(now)
        .bind(now + ACTIVE_ROOM_TTL_SECONDS)
        .bind(room_id)
        .bind(revision)
        .execute(&mut *transaction)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(StoreError::Conflict);
        }
        sqlx::query(
            "INSERT INTO actions (room_id, revision, actor, action_json, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(room_id)
        .bind(next_revision)
        .bind(actor.to_string())
        .bind(serde_json::to_string(&action)?)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.load_room(room_id).await
    }

    /// Updates a player's coarse presence timestamp at most once per five seconds.
    ///
    /// # Errors
    ///
    /// Returns a database error when the update cannot be executed.
    pub async fn touch(&self, room_id: &str, seat: Seat) -> Result<(), StoreError> {
        let now = now_epoch();
        let column = match seat {
            Seat::One => "player_one_seen_at",
            Seat::Two => "player_two_seen_at",
        };
        let query = format!(
            "UPDATE rooms SET {column} = ? WHERE id = ? AND ({column} IS NULL OR {column} < ?)"
        );
        sqlx::query(&query)
            .bind(now)
            .bind(room_id)
            .bind(now - 5)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Deletes expired rooms and their cascaded action logs.
    ///
    /// # Errors
    ///
    /// Returns a database error when cleanup cannot be executed.
    pub async fn cleanup_expired(&self) -> Result<u64, StoreError> {
        let result = sqlx::query("DELETE FROM rooms WHERE expires_at <= ?")
            .bind(now_epoch())
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

fn row_to_room(row: &sqlx::sqlite::SqliteRow) -> Result<Room, StoreError> {
    let mode = match row.try_get::<String, _>("mode")?.as_str() {
        "computer" => RoomMode::Computer,
        "friend" => RoomMode::Friend,
        _ => return Err(StoreError::NotFound),
    };
    Ok(Room {
        id: row.try_get("id")?,
        mode,
        status: row.try_get("status")?,
        revision: row.try_get("revision")?,
        state: serde_json::from_str(&row.try_get::<String, _>("state_json")?)?,
        player_names: [
            Some(row.try_get("player_one_name")?),
            row.try_get("player_two_name")?,
        ],
        player_token_hashes: [
            Some(row.try_get("player_one_token_hash")?),
            row.try_get("player_two_token_hash")?,
        ],
        last_seen_at: [
            Some(row.try_get("player_one_seen_at")?),
            row.try_get("player_two_seen_at")?,
        ],
        updated_at: row.try_get("updated_at")?,
    })
}

#[must_use]
pub fn hash_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[must_use]
pub fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ScoreVisibility;

    #[tokio::test]
    async fn room_round_trip_and_atomic_action() {
        let temp = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", temp.path().join("test.db").display());
        let store = Store::connect(&url).await.unwrap();
        let state = MatchState::new(7, ScoreVisibility::Visible);
        store
            .create_room(NewRoom {
                id: "ABC123",
                mode: RoomMode::Computer,
                state: &state,
                player_one_name: "Ada",
                player_one_token: "secret",
                player_two_name: Some("Computer"),
                player_two_token: Some("bot"),
            })
            .await
            .unwrap();
        let room = store.load_room("ABC123").await.unwrap();
        assert_eq!(room.seat_for_token("secret"), Some(Seat::One));
        let actor = room.state.deal.active_player();
        let card = room.state.deal.hands[actor.index()][0];
        let updated = store
            .apply_action(
                "ABC123",
                0,
                actor,
                Action::Play {
                    card,
                    announce_marriage: false,
                    declare: false,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.revision, 1);
        assert_eq!(updated.state.deal.played_cards, vec![card]);
        assert!(matches!(
            store
                .apply_action("ABC123", 0, actor, Action::Declare)
                .await,
            Err(StoreError::Conflict)
        ));
    }
}
