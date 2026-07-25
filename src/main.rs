use std::env;
use std::fmt::Write as _;
use std::time::Duration;

use rand::Rng;
use serde::Deserialize;
use sixty_six::bot::{Observation, choose_action};
use sixty_six::game::{Action, Card, DealEndReason, MatchState, Rank, ScoreVisibility, Seat, Suit};
use sixty_six::store::{NewRoom, Room, RoomMode, Store, StoreError, now_epoch};
use topcoat::{
    Result,
    context::{Cx, app_context},
    cookie::{Cookie, Cookies, RouterBuilderCookieExt, SameSite, cookies},
    htmx::hx_request,
    router::{
        Form, IntoResponse, Response, Router, RouterBuilderDiscoverExt, SeeOther, Slot, layout,
        not_found, page, path_param, route, see_other,
    },
    session::{Config as SessionConfig, RouterBuilderSessionExt},
    view::{View, view},
};

const HTMX_INTEGRITY: &str =
    "sha384-H5SrcfygHmAuTDZphMHqBJLc3FhssKjG7w/CeCpFReSfwBWDTKpkzPP8c+cLsK+V";

#[derive(Clone)]
struct App {
    store: Store,
    public_base_url: String,
    secure_cookies: bool,
}

#[derive(Deserialize)]
struct CreateGame {
    nickname: String,
    score_visibility: String,
}

#[derive(Deserialize)]
struct JoinGame {
    nickname: String,
}

#[derive(Deserialize)]
struct JoinCode {
    room: String,
}

#[derive(Deserialize)]
struct GameAction {
    revision: i64,
    action: String,
    card: Option<String>,
    intent: Option<String>,
}

#[derive(Deserialize)]
struct BotTurn {
    revision: i64,
}

#[topcoat::router::path_param]
struct GameId(str);

#[tokio::main]
async fn main() {
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://sixty-six.db".to_owned());
    let public_base_url = env::var("PUBLIC_BASE_URL")
        .unwrap_or_else(|_| "http://localhost:3000".to_owned())
        .trim_end_matches('/')
        .to_owned();
    let secure_cookies = env::var("APP_ENV").is_ok_and(|value| value == "production");
    let store = Store::connect(&database_url)
        .await
        .expect("connect to SQLite and apply schema");

    let cleanup_store = store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_hours(1));
        loop {
            interval.tick().await;
            let _ = cleanup_store.cleanup_expired().await;
        }
    });

    let app = App {
        store,
        public_base_url,
        secure_cookies,
    };
    let router = Router::builder()
        .discover()
        .cookies()
        .sessions(SessionConfig::default())
        .app_context(app)
        .build();
    topcoat::start(router).await.expect("serve Sixty-Six");
}

fn app(cx: &Cx) -> &App {
    app_context(cx)
}

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result {
    if hx_request(cx) {
        return slot.await;
    }
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
                <meta name="theme-color" content="#12382d">
                <meta
                    name="description"
                    content="Play the classic two-player card game Sixty-Six against a clever computer or a friend."
                >
                <title>"Sixty-Six · Play online"</title>
                <link rel="stylesheet" href="/styles.css">
                <script
                    defer="true"
                    src="https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js"
                    integrity=(HTMX_INTEGRITY)
                    crossorigin="anonymous"
                ></script>
            </head>
            <body>
                <header class="site-header">
                    <a class="brand" href="/" aria-label="Sixty-Six home">
                        <span class="brand-mark">"66"</span>
                        <span>"Sixty-Six"</span>
                    </a>
                    <a class="quiet-link" href="/rules">"How to play"</a>
                </header>
                <main>(slot.await?)</main>
                <footer>
                    <span>"24 cards · 2 players · first to 7"</span>
                </footer>
            </body>
        </html>
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    view! {
        <section class="home-shell">
            <div class="hero-copy">
                <p class="eyebrow">"A sharp little trick-taking game"</p>
                <h1>"Sixty-Six, without the table."</h1>
                <p class="lede">
                    "Play a full match against a thoughtful computer or send one private link to a friend. No account, no download."
                </p>
                <div class="hero-cards" aria-hidden="true">
                    (decorative_card(cx, Card::new(Suit::Hearts, Rank::Ace), "tilt-left").await?)
                    (decorative_card(cx, Card::new(Suit::Spades, Rank::Ten), "tilt-mid").await?)
                    (decorative_card(cx, Card::new(Suit::Diamonds, Rank::King), "tilt-right").await?)
                </div>
            </div>
            <div class="setup-card">
                <h2>"Start a match"</h2>
                <form method="post" class="setup-form">
                    <label for="nickname">"Your name"</label>
                    <input
                        id="nickname"
                        name="nickname"
                        maxlength="20"
                        autocomplete="nickname"
                        placeholder="Player"
                    >
                    <fieldset>
                        <legend>"Card-point display"</legend>
                        <label class="choice">
                            <input
                                type="radio"
                                name="score_visibility"
                                value="visible"
                                checked="true"
                            >
                            <span>
                                <strong>"Visible"</strong>
                                <small>"Keep a running count for both players"</small>
                            </span>
                        </label>
                        <label class="choice">
                            <input
                                type="radio"
                                name="score_visibility"
                                value="traditional"
                            >
                            <span>
                                <strong>"Traditional"</strong>
                                <small>"Remember your own captured points"</small>
                            </span>
                        </label>
                    </fieldset>
                    <div class="start-actions">
                        <button class="primary" type="submit" formaction="/games/computer">
                            <span>"Play computer"</span><span aria-hidden="true">"→"</span>
                        </button>
                        <button class="secondary" type="submit" formaction="/games/friend">
                            <span>"Create friend game"</span><span aria-hidden="true">"↗"</span>
                        </button>
                    </div>
                </form>
                <div class="join-divider"><span>"or join a room"</span></div>
                <form method="get" action="/join" class="join-form">
                    <label class="sr-only" for="room">"Room code or link"</label>
                    <input id="room" name="room" placeholder="Room code or link" required="true">
                    <button type="submit">"Join"</button>
                </form>
            </div>
        </section>
    }
}

#[page("/rules")]
async fn rules() -> Result {
    view! {
        <article class="rules-page">
            <p class="eyebrow">"The 24-card game"</p>
            <h1>"How to play Sixty-Six"</h1>
            <p class="lede">
                "Win tricks, announce marriages, and be the first to declare 66 card points. A match ends when one player reaches 7 game points."
            </p>
            <section>
                <h2>"The deck"</h2>
                <p>
                    "Each suit contains Ace, Ten, King, Queen, Jack, and Nine. They rank in that order and are worth 11, 10, 4, 3, 2, and 0 card points."
                </p>
            </section>
            <section>
                <h2>"Open stock"</h2>
                <p>
                    "Each player starts with six cards. While the stock is open, you may follow with any card. The trick winner draws first, then leads the next trick."
                </p>
            </section>
            <section>
                <h2>"Marriages and trump exchange"</h2>
                <p>
                    "Lead a King or Queen while holding its partner to announce 20 points, or 40 in trumps. Once you have won a trick, you may exchange the trump Nine for the face-up trump while leading."
                </p>
            </section>
            <section>
                <h2>"Closing and strict play"</h2>
                <p>
                    "The player on lead may close the stock. Once closed—or once the stock is exhausted—you must follow suit and head the trick when able; otherwise you must trump. Closing is a promise: fail to reach 66 and your opponent scores at least 2 game points."
                </p>
            </section>
            <section>
                <h2>"Declare 66"</h2>
                <p>
                    "Declaration is always explicit. A correct declaration wins 1, 2, or 3 game points depending on the opponent’s tricks and card points. A false declaration awards the deal to the opponent."
                </p>
            </section>
            <a class="primary button-link" href="/">"Start a match"</a>
        </article>
    }
}

#[route(GET "/styles.css")]
async fn styles() -> Result<([(&'static str, &'static str); 2], &'static str)> {
    Ok((
        [
            ("content-type", "text/css; charset=utf-8"),
            ("cache-control", "public, max-age=3600"),
        ],
        include_str!("styles.css"),
    ))
}

#[route(GET "/health")]
async fn health() -> Result<&'static str> {
    Ok("ok")
}

#[route(GET "/join")]
async fn join_by_code(Form(form): Form<JoinCode>) -> Result<SeeOther> {
    let code = room_code_from_input(&form.room);
    if code.is_empty() {
        return Ok(see_other("/"));
    }
    Ok(see_other(&format!("/games/{code}")))
}

#[route(POST "/games/computer")]
async fn create_computer(cx: &Cx, Form(form): Form<CreateGame>) -> Result<SeeOther> {
    create_room(cx, form, RoomMode::Computer).await
}

#[route(POST "/games/friend")]
async fn create_friend(cx: &Cx, Form(form): Form<CreateGame>) -> Result<SeeOther> {
    create_room(cx, form, RoomMode::Friend).await
}

async fn create_room(cx: &Cx, form: CreateGame, mode: RoomMode) -> Result<SeeOther> {
    let room_id = random_room_id();
    let player_token = random_token();
    let bot_token = random_token();
    let visibility = if form.score_visibility == "traditional" {
        ScoreVisibility::Traditional
    } else {
        ScoreVisibility::Visible
    };
    let state = MatchState::new(rand::rng().random(), visibility);
    let nickname = clean_name(&form.nickname);
    app(cx)
        .store
        .create_room(NewRoom {
            id: &room_id,
            mode,
            state: &state,
            player_one_name: &nickname,
            player_one_token: &player_token,
            player_two_name: (mode == RoomMode::Computer).then_some("Computer"),
            player_two_token: (mode == RoomMode::Computer).then_some(bot_token.as_str()),
        })
        .await?;
    set_seat_cookie(cx, &room_id, &player_token);
    Ok(see_other(&format!("/games/{room_id}")))
}

#[page("/games/{game_id}")]
async fn game_page(cx: &Cx) -> Result {
    let room_id = path_param::<GameId>(cx);
    let room = load_room_or_404(cx, room_id).await?;
    let seat = current_seat(cx, &room);
    if let Some(seat) = seat {
        let _ = app(cx).store.touch(room_id, seat).await;
        return board_view(cx, &room, seat, None).await;
    }
    if room.mode == RoomMode::Friend && !room.has_second_player() {
        return join_room_view(cx, &room).await;
    }
    unavailable_room_view(cx, &room).await
}

#[route(POST "/games/{game_id}/join")]
async fn join_room(cx: &Cx, Form(form): Form<JoinGame>) -> Result<SeeOther> {
    let room_id = path_param::<GameId>(cx);
    let token = random_token();
    app(cx)
        .store
        .join_friend(room_id, &clean_name(&form.nickname), &token)
        .await?;
    set_seat_cookie(cx, room_id, &token);
    Ok(see_other(&format!("/games/{room_id}")))
}

#[route(GET "/games/{game_id}/board")]
async fn board(cx: &Cx) -> Result<View> {
    let room_id = path_param::<GameId>(cx);
    let room = load_room_or_404(cx, room_id).await?;
    let Some(seat) = current_seat(cx, &room) else {
        return unavailable_room_view(cx, &room).await;
    };
    let _ = app(cx).store.touch(room_id, seat).await;
    board_view(cx, &room, seat, None).await
}

#[route(POST "/games/{game_id}/action")]
async fn game_action(cx: &Cx, Form(form): Form<GameAction>) -> Result<Response> {
    let room_id = path_param::<GameId>(cx);
    let room = load_room_or_404(cx, room_id).await?;
    let Some(seat) = current_seat(cx, &room) else {
        return not_found().into_response(cx);
    };
    let action = parse_action(&form)?;
    let result = app(cx)
        .store
        .apply_action(room_id, form.revision, seat, action)
        .await;
    action_response(cx, room_id, seat, result).await
}

#[route(POST "/games/{game_id}/bot")]
async fn bot_turn(cx: &Cx, Form(form): Form<BotTurn>) -> Result<Response> {
    let room_id = path_param::<GameId>(cx);
    let room = load_room_or_404(cx, room_id).await?;
    if room.mode != RoomMode::Computer
        || current_seat(cx, &room) != Some(Seat::One)
        || room.state.deal.result.is_some()
        || room.state.deal.active_player() != Seat::Two
    {
        return action_response(cx, room_id, Seat::One, Ok(room)).await;
    }
    let observation = Observation::for_player(&room.state, Seat::Two);
    let action = choose_action(
        &observation,
        u64::try_from(room.revision).unwrap_or_default(),
    );
    let result = app(cx)
        .store
        .apply_action(room_id, form.revision, Seat::Two, action)
        .await;
    action_response(cx, room_id, Seat::One, result).await
}

async fn action_response(
    cx: &Cx,
    room_id: &str,
    seat: Seat,
    result: std::result::Result<Room, StoreError>,
) -> Result<Response> {
    if !hx_request(cx) {
        return see_other(&format!("/games/{room_id}")).into_response(cx);
    }
    let (room, notice): (Room, Option<String>) = match result {
        Ok(room) => (room, None),
        Err(StoreError::Conflict) => (
            load_room_or_404(cx, room_id).await?,
            Some("The game moved on. Your board has been refreshed.".to_owned()),
        ),
        Err(StoreError::Rule(error)) => (
            load_room_or_404(cx, room_id).await?,
            Some(error.to_string()),
        ),
        Err(error) => return Err(error.into()),
    };
    board_view(cx, &room, seat, notice.as_deref())
        .await?
        .into_response(cx)
}

fn parse_action(form: &GameAction) -> Result<Action> {
    let action = match form.action.as_str() {
        "play" => {
            let card = form
                .card
                .as_deref()
                .and_then(Card::parse)
                .ok_or_else(|| StoreError::Conflict)?;
            let intent = form.intent.as_deref().unwrap_or("play");
            Action::Play {
                card,
                announce_marriage: matches!(intent, "marriage" | "marriage_declare"),
                declare: intent == "marriage_declare",
            }
        }
        "exchange" => Action::ExchangeTrump,
        "close" => Action::CloseStock,
        "declare" => Action::Declare,
        "next" => Action::NextDeal,
        _ => return Err(StoreError::Conflict.into()),
    };
    Ok(action)
}

async fn load_room_or_404(cx: &Cx, room_id: &str) -> Result<Room> {
    match app(cx).store.load_room(room_id).await {
        Ok(room) => Ok(room),
        Err(StoreError::NotFound) => Err(not_found().into()),
        Err(error) => Err(error.into()),
    }
}

fn current_seat(cx: &Cx, room: &Room) -> Option<Seat> {
    let token = cookies(cx)
        .get("seat")
        .map(|cookie| cookie.value().to_owned())?;
    room.seat_for_token(&token)
}

fn set_seat_cookie(cx: &Cx, room_id: &str, token: &str) {
    let path = format!("/games/{room_id}");
    cookies(cx).add(
        Cookie::build(("seat", token.to_owned()))
            .path(path)
            .http_only(true)
            .secure(app(cx).secure_cookies)
            .same_site(SameSite::Lax)
            .max_age(topcoat::cookie::time::Duration::days(7))
            .build(),
    );
}

async fn join_room_view(__cx: &Cx, room: &Room) -> Result {
    let host = room.player_names[0].as_deref().unwrap_or("A friend");
    let scoring = score_mode_label(room.state.settings.score_visibility);
    view! {
        <section class="room-gate">
            <div class="gate-mark">"66"</div>
            <p class="eyebrow">"Private room"</p>
            <h1>((host, " invited you"))</h1>
            <p>((scoring, " scoring · match to 7"))</p>
            <form method="post" action=(("/games/", room.id.as_str(), "/join")) class="gate-form">
                <label for="nickname">"Your name"</label>
                <input
                    id="nickname"
                    name="nickname"
                    maxlength="20"
                    autocomplete="nickname"
                    placeholder="Player two"
                >
                <button class="primary" type="submit">"Take your seat"</button>
            </form>
        </section>
    }
}

async fn unavailable_room_view(__cx: &Cx, room: &Room) -> Result {
    view! {
        <section class="room-gate">
            <div class="gate-mark muted">"66"</div>
            <p class="eyebrow">"Private room"</p>
            <h1>"This table is full"</h1>
            <p>"Only the two seated players can see or play this match."</p>
            <a class="primary button-link" href="/">"Start another game"</a>
            <small>(("Room ", room.id.as_str()))</small>
        </section>
    }
}

#[allow(clippy::too_many_lines)]
async fn board_view(__cx: &Cx, room: &Room, viewer: Seat, notice: Option<&str>) -> Result {
    let state = &room.state;
    let deal = &state.deal;
    let opponent = viewer.other();
    let viewer_name = room.player_names[viewer.index()]
        .as_deref()
        .unwrap_or("You");
    let opponent_name = room.player_names[opponent.index()]
        .as_deref()
        .unwrap_or("Opponent");
    let viewer_turn = deal.result.is_none() && deal.active_player() == viewer;
    let waiting_for_friend = room.mode == RoomMode::Friend && !room.has_second_player();
    let is_bot_turn = room.mode == RoomMode::Computer
        && deal.result.is_none()
        && deal.active_player() == Seat::Two;
    let poll = waiting_for_friend || (room.mode == RoomMode::Friend && !viewer_turn);
    let game_url = format!("{}/games/{}", app(__cx).public_base_url, room.id);
    let action_url = format!("/games/{}/action", room.id);
    let board_url = format!("/games/{}/board", room.id);
    let bot_url = format!("/games/{}/bot", room.id);
    let status = board_status(room, viewer);
    let status_class = if viewer_turn {
        "turn-status yours"
    } else {
        "turn-status"
    };
    let mut hand = deal.hands[viewer.index()].clone();
    hand.sort_by_key(|card| (card.suit, card.rank.strength()));
    let legal = deal.legal_cards(viewer);
    let show_scores =
        state.settings.score_visibility == ScoreVisibility::Visible || deal.result.is_some();
    let can_declare = viewer_turn
        && deal.trick.is_empty()
        && (state.settings.score_visibility == ScoreVisibility::Traditional
            || deal.card_points[viewer.index()] >= 66);

    view! {
        <section
            id="game-board"
            class="game-shell"
            if poll {
                hx-get=(board_url)
                hx-trigger="every 1s"
                hx-swap="outerHTML"
            }
        >
            if is_bot_turn {
                <form
                    class="bot-trigger"
                    hx-post=(bot_url)
                    hx-trigger="load delay:650ms"
                    hx-target="#game-board"
                    hx-swap="outerHTML"
                >
                    <input type="hidden" name="revision" value=(room.revision)>
                </form>
            }
            if let Some(message) = notice {
                <div class="notice" role="status">(message)</div>
            }
            <div class="game-topbar">
                <div>
                    <span class="room-code">(("ROOM ", room.id.as_str()))</span>
                    <span class="score-mode">
                        if state.settings.score_visibility == ScoreVisibility::Visible {
                            "visible count"
                        } else {
                            "traditional count"
                        }
                    </span>
                </div>
                <div class="match-score" aria-label="Match score">
                    <span>((viewer_name, " ", state.match_points[viewer.index()]))</span>
                    <span class="score-separator">"—"</span>
                    <span>((state.match_points[opponent.index()], " ", opponent_name))</span>
                </div>
            </div>

            if waiting_for_friend {
                <div class="waiting-panel">
                    <div class="waiting-pip" aria-hidden="true"></div>
                    <h1>"Waiting for your friend"</h1>
                    <p>"Share this private link. The first person to open it takes the other seat."</p>
                    <label for="invite-link">"Invite link"</label>
                    <input id="invite-link" readonly="true" value=(game_url)>
                    <small>"This page updates when they arrive."</small>
                </div>
            } else {
                <div class="table" data-trump=(deal.trump.name())>
                    <section class="opponent-zone" aria-label="Opponent">
                        <div class="player-meta">
                            <div>
                                <span class="player-name">(opponent_name)</span>
                                <span class="presence">
                                    if room.mode == RoomMode::Computer {
                                        "computer"
                                    } else if room.last_seen_at[opponent.index()]
                                        .is_some_and(|seen| now_epoch() - seen < 8)
                                    {
                                        "online"
                                    } else {
                                        "away"
                                    }
                                </span>
                            </div>
                            <span class="card-points">
                                if show_scores {
                                    ((deal.card_points[opponent.index()], " pts"))
                                } else {
                                    "points hidden"
                                }
                            </span>
                        </div>
                        <div class="opponent-hand" aria-label=(format!("{} cards", deal.hands[opponent.index()].len()))>
                            for _ in 0..deal.hands[opponent.index()].len() {
                                (card_back(__cx, "mini").await?)
                            }
                        </div>
                    </section>

                    <section class="table-center" aria-label="Current trick and stock">
                        <div class="trick-area">
                            <span class="area-label">
                                if deal.trick.is_empty() && deal.last_trick.is_some() {
                                    "last trick"
                                } else {
                                    "current trick"
                                }
                            </span>
                            <div class="trick-slots">
                                if deal.trick.is_empty() {
                                    if let Some(last) = &deal.last_trick {
                                        (table_card(__cx, last.cards[0].card, "played faded").await?)
                                        (table_card(__cx, last.cards[1].card, "played faded").await?)
                                    } else {
                                        <div class="empty-card"></div>
                                        <div class="empty-card"></div>
                                    }
                                } else {
                                    for play in &deal.trick {
                                        (table_card(__cx, play.card, "played").await?)
                                    }
                                    if deal.trick.len() == 1 {
                                        <div class="empty-card"></div>
                                    }
                                }
                            </div>
                        </div>
                        <div class="stock-area">
                            <span class="area-label">
                                if deal.closed_by.is_some() {
                                    "stock closed"
                                } else {
                                    ((deal.stock_count(), " in stock"))
                                }
                            </span>
                            <div class="stock-cards">
                                if deal.stock_count() == 0 {
                                    <div class="empty-stock">"empty"</div>
                                } else if deal.closed_by.is_some() {
                                    (card_back(__cx, "stock closed").await?)
                                    <span class="trump-marker">((deal.trump.symbol(), " trump"))</span>
                                } else {
                                    (card_back(__cx, "stock").await?)
                                    if let Some(trump_card) = deal.trump_card {
                                        (table_card(__cx, trump_card, "trump-card").await?)
                                        <span class="trump-marker">
                                            ((deal.trump.symbol(), " trump"))
                                        </span>
                                    }
                                }
                            </div>
                        </div>
                    </section>

                    <section class="you-zone" aria-label="Your hand">
                        <div class=(status_class) role="status">
                            <span class="turn-dot" aria-hidden="true"></span>
                            <strong>(status)</strong>
                        </div>
                        <div class="player-meta you-meta">
                            <span class="player-name">((viewer_name, " · you"))</span>
                            <span class="card-points">
                                if show_scores {
                                    ((deal.card_points[viewer.index()], " card points"))
                                } else {
                                    "count your tricks"
                                }
                            </span>
                        </div>

                        if let Some(result) = deal.result {
                            <div class="deal-result">
                                <p class="eyebrow">(("Deal ", state.deal_number, " complete"))</p>
                                <h2>
                                    if result.winner == viewer {
                                        "You won the deal"
                                    } else {
                                        ((opponent_name, " won the deal"))
                                    }
                                </h2>
                                <p>
                                    ((result.game_points, if result.game_points == 1 { " game point · " } else { " game points · " }, end_reason(result.reason)))
                                </p>
                                if let Some(match_winner) = state.winner {
                                    <strong class="match-winner">
                                        if match_winner == viewer {
                                            "You won the match!"
                                        } else {
                                            ((opponent_name, " won the match"))
                                        }
                                    </strong>
                                    <a class="primary button-link" href="/">"Play again"</a>
                                } else {
                                    (simple_action_form(__cx, &action_url, room.revision, "next", "Deal again", "primary").await?)
                                }
                            </div>
                        } else {
                            <div class="hand" aria-label="Your cards">
                                for card in hand {
                                    {
                                        let is_legal = viewer_turn && legal.contains(&card);
                                        let marriage = deal.can_announce_marriage(viewer, card);
                                        let marriage_value = deal.marriage_value(card).unwrap_or_default();
                                        let can_marriage_declare = marriage
                                            && (state.settings.score_visibility == ScoreVisibility::Traditional
                                                || (deal.tricks_won[viewer.index()] > 0
                                                    && deal.card_points[viewer.index()] + marriage_value >= 66));
                                        (playable_card(
                                            __cx,
                                            &action_url,
                                            room.revision,
                                            card,
                                            is_legal,
                                            marriage,
                                            can_marriage_declare,
                                            marriage_value,
                                        ).await?)
                                    }
                                }
                            </div>
                            if viewer_turn {
                                <div class="table-actions">
                                    if deal.can_exchange_trump(viewer) {
                                        (simple_action_form(__cx, &action_url, room.revision, "exchange", "Exchange trump 9", "table-action").await?)
                                    }
                                    if deal.can_close_stock(viewer) {
                                        (simple_action_form(__cx, &action_url, room.revision, "close", "Close stock", "table-action").await?)
                                    }
                                    if can_declare {
                                        (simple_action_form(__cx, &action_url, room.revision, "declare", "Declare 66", "declare-action").await?)
                                    }
                                </div>
                            }
                        }
                    </section>
                </div>
            }
        </section>
    }
}

#[allow(clippy::too_many_arguments)]
async fn playable_card(
    __cx: &Cx,
    action_url: &str,
    revision: i64,
    card: Card,
    legal: bool,
    marriage: bool,
    can_marriage_declare: bool,
    marriage_value: u16,
) -> Result {
    let color = if card.suit.is_red() { "red" } else { "black" };
    let class = if legal {
        format!("card-face hand-card {color} legal")
    } else {
        format!("card-face hand-card {color}")
    };
    view! {
        <form
            method="post"
            action=(action_url)
            hx-post=(action_url)
            hx-target="#game-board"
            hx-swap="outerHTML"
            class="card-form"
        >
            <input type="hidden" name="revision" value=(revision)>
            <input type="hidden" name="action" value="play">
            <input type="hidden" name="card" value=(card.ascii_code())>
            <button
                type="submit"
                name="intent"
                value="play"
                class=(class)
                aria-label=(format!("Play {}", card.accessible_name()))
                disabled=(!legal)
            >
                <span class="corner">
                    <strong>(card.rank.symbol())</strong><span>(card.suit.symbol())</span>
                </span>
                <span class="center-suit">(card.suit.symbol())</span>
                <span class="corner bottom">
                    <strong>(card.rank.symbol())</strong><span>(card.suit.symbol())</span>
                </span>
            </button>
            if marriage && legal {
                <button
                    class="marriage-action"
                    type="submit"
                    name="intent"
                    value="marriage"
                    aria-label=(format!("Play {} and announce a {} point marriage", card.accessible_name(), marriage_value))
                >
                    (("+", marriage_value))
                </button>
                if can_marriage_declare {
                    <button
                        class="marriage-declare"
                        type="submit"
                        name="intent"
                        value="marriage_declare"
                    >
                        "marry + 66"
                    </button>
                }
            }
        </form>
    }
}

async fn simple_action_form(
    __cx: &Cx,
    action_url: &str,
    revision: i64,
    action: &str,
    label: &str,
    class: &str,
) -> Result {
    view! {
        <form
            method="post"
            action=(action_url)
            hx-post=(action_url)
            hx-target="#game-board"
            hx-swap="outerHTML"
        >
            <input type="hidden" name="revision" value=(revision)>
            <input type="hidden" name="action" value=(action)>
            <button type="submit" class=(class)>(label)</button>
        </form>
    }
}

async fn table_card(__cx: &Cx, card: Card, extra_class: &str) -> Result {
    let color = if card.suit.is_red() { "red" } else { "black" };
    view! {
        <div
            class=(format!("card-face table-card {color} {extra_class}"))
            aria-label=(card.accessible_name())
        >
            <span class="corner">
                <strong>(card.rank.symbol())</strong><span>(card.suit.symbol())</span>
            </span>
            <span class="center-suit">(card.suit.symbol())</span>
        </div>
    }
}

async fn decorative_card(__cx: &Cx, card: Card, extra_class: &str) -> Result {
    let color = if card.suit.is_red() { "red" } else { "black" };
    view! {
        <div class=(format!("card-face decorative {color} {extra_class}"))>
            <span class="corner">
                <strong>(card.rank.symbol())</strong><span>(card.suit.symbol())</span>
            </span>
            <span class="center-suit">(card.suit.symbol())</span>
        </div>
    }
}

async fn card_back(__cx: &Cx, extra_class: &str) -> Result {
    view! {
        <div class=(format!("card-back {extra_class}")) aria-hidden="true">
            <span>"66"</span>
        </div>
    }
}

fn board_status(room: &Room, viewer: Seat) -> String {
    let state = &room.state;
    if let Some(winner) = state.winner {
        return if winner == viewer {
            "Match won".to_owned()
        } else {
            "Match complete".to_owned()
        };
    }
    if state.deal.result.is_some() {
        return "Deal complete".to_owned();
    }
    let active = state.deal.active_player();
    if active == viewer {
        "Your turn".to_owned()
    } else if room.mode == RoomMode::Computer {
        "Computer is thinking…".to_owned()
    } else {
        format!(
            "Waiting for {}",
            room.player_names[active.index()]
                .as_deref()
                .unwrap_or("opponent")
        )
    }
}

fn end_reason(reason: DealEndReason) -> &'static str {
    match reason {
        DealEndReason::Declared => "declared 66",
        DealEndReason::IncorrectDeclaration => "incorrect declaration",
        DealEndReason::ClosedStockFailed => "failed closure",
        DealEndReason::LastTrick => "last trick",
    }
}

fn score_mode_label(visibility: ScoreVisibility) -> &'static str {
    match visibility {
        ScoreVisibility::Visible => "Visible",
        ScoreVisibility::Traditional => "Traditional",
    }
}

fn clean_name(input: &str) -> String {
    let cleaned: String = input
        .trim()
        .chars()
        .filter(|character| !character.is_control())
        .take(20)
        .collect();
    if cleaned.is_empty() {
        "Player".to_owned()
    } else {
        cleaned
    }
}

fn room_code_from_input(input: &str) -> String {
    input
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_uppercase())
        .take(8)
        .collect()
}

fn random_room_id() -> String {
    const ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTUVWXYZ";
    let mut rng = rand::rng();
    (0..8)
        .map(|_| ALPHABET[rng.random_range(0..ALPHABET.len())] as char)
        .collect()
}

fn random_token() -> String {
    let mut token = String::with_capacity(64);
    let mut rng = rand::rng();
    for _ in 0..32 {
        write!(&mut token, "{:02x}", rng.random::<u8>()).expect("write to String");
    }
    token
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_names_and_defaults_empty_values() {
        assert_eq!(clean_name("  Ada\n"), "Ada");
        assert_eq!(clean_name(" \t "), "Player");
        assert_eq!(clean_name(&"x".repeat(30)).chars().count(), 20);
    }

    #[test]
    fn extracts_room_codes_from_links() {
        assert_eq!(
            room_code_from_input("https://sixty-six.example/games/a2bc-99z/"),
            "A2BC99Z"
        );
        assert_eq!(room_code_from_input(" abc123 "), "ABC123");
    }

    #[test]
    fn random_identifiers_have_expected_shape() {
        let room = random_room_id();
        let token = random_token();
        assert_eq!(room.len(), 8);
        assert_eq!(token.len(), 64);
        assert!(
            room.chars()
                .all(|character| character.is_ascii_alphanumeric())
        );
        assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
    }
}
