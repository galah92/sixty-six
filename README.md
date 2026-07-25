# Sixty-Six

A minimal, mobile-first implementation of the classic 24-card game. Play a
match to seven game points against a fair-information computer or a friend with
one private room link.

## Stack

- Rust 1.95+
- Topcoat 0.4 for routing and server-rendered views
- HTMX 2.0.10 for HTML fragment updates and room polling
- SQLx and SQLite for persistent, atomic game state
- Plain CSS; no handwritten browser JavaScript

Topcoat is intentionally pinned because the framework is experimental and may
make breaking changes. The rules engine, bot observation, and store remain
independent of Topcoat.

## Run locally

```bash
DATABASE_URL=sqlite://local.db \
PUBLIC_BASE_URL=http://localhost:3000 \
cargo run
```

Open <http://localhost:3000>.

## Validate

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The test suite includes hundreds of complete seeded matches, card-conservation
invariants, strict-play rules, scoring transitions, bot information boundaries,
and atomic SQLite revision checks.

## Production model

The Fly.io configuration intentionally runs one Machine with one attached
volume. This keeps SQLite simple and gives the application a single writer.
Fly snapshots are retained for 14 days. There is brief downtime during a
deployment and an acknowledged risk of losing an active match after a volume
host failure; v1 stores no accounts, purchases, or rankings.

Rooms waiting for a friend expire after 24 hours. Active and completed rooms
expire after seven days. The append-only diagnostic event ledger is retained
for 90 days and stores the action plus authoritative state immediately before
and after it. Legacy actions created before the event ledger retain their
actor, revision, action, and timestamp, but do not have state snapshots.

## Game diagnostics

Every accepted or rejected action emits a structured log with the room ID,
revision, actor, action type, points, pending marriages, trick counts, current
leader, and completion state. Search live Fly logs by room code:

```bash
fly logs --app sixty-six-card-game | rg ROOM_CODE
```

For an exact retained timeline, run the internal inspection command. It prints
JSON and is not exposed over HTTP:

```bash
DATABASE_URL=sqlite://local.db cargo run -- inspect ROOM_CODE
fly ssh console --app sixty-six-card-game \
  --command "/usr/local/bin/sixty-six inspect ROOM_CODE"
```

## Deploy

Authenticate `flyctl`, ensure the globally unique app name in `fly.toml` is
available, then run:

```bash
fly apps create sixty-six-card-game
fly volumes create sixty_six_data --region fra --size 1 --snapshot-retention 14
fly deploy --remote-only
fly scale count 1
```

The service health endpoint is `/health`.
