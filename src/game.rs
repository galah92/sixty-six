use std::collections::VecDeque;
use std::fmt;

use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MATCH_TARGET: u8 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seat {
    One,
    Two,
}

impl Seat {
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Self::One => 0,
            Self::Two => 1,
        }
    }

    #[must_use]
    pub const fn other(self) -> Self {
        match self {
            Self::One => Self::Two,
            Self::Two => Self::One,
        }
    }
}

impl fmt::Display for Seat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::One => "one",
            Self::Two => "two",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Self; 4] = [Self::Clubs, Self::Diamonds, Self::Hearts, Self::Spades];

    #[must_use]
    pub const fn symbol(self) -> char {
        match self {
            Self::Clubs => '♣',
            Self::Diamonds => '♦',
            Self::Hearts => '♥',
            Self::Spades => '♠',
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clubs => "clubs",
            Self::Diamonds => "diamonds",
            Self::Hearts => "hearts",
            Self::Spades => "spades",
        }
    }

    #[must_use]
    pub const fn is_red(self) -> bool {
        matches!(self, Self::Diamonds | Self::Hearts)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rank {
    Nine,
    Jack,
    Queen,
    King,
    Ten,
    Ace,
}

impl Rank {
    pub const ALL: [Self; 6] = [
        Self::Nine,
        Self::Jack,
        Self::Queen,
        Self::King,
        Self::Ten,
        Self::Ace,
    ];

    #[must_use]
    pub const fn points(self) -> u16 {
        match self {
            Self::Nine => 0,
            Self::Jack => 2,
            Self::Queen => 3,
            Self::King => 4,
            Self::Ten => 10,
            Self::Ace => 11,
        }
    }

    #[must_use]
    pub const fn strength(self) -> u8 {
        match self {
            Self::Nine => 0,
            Self::Jack => 1,
            Self::Queen => 2,
            Self::King => 3,
            Self::Ten => 4,
            Self::Ace => 5,
        }
    }

    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Nine => "9",
            Self::Jack => "J",
            Self::Queen => "Q",
            Self::King => "K",
            Self::Ten => "10",
            Self::Ace => "A",
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Nine => "nine",
            Self::Jack => "jack",
            Self::Queen => "queen",
            Self::King => "king",
            Self::Ten => "ten",
            Self::Ace => "ace",
        }
    }

    #[must_use]
    pub const fn marriage_partner(self) -> Option<Self> {
        match self {
            Self::King => Some(Self::Queen),
            Self::Queen => Some(Self::King),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

impl Card {
    #[must_use]
    pub const fn new(suit: Suit, rank: Rank) -> Self {
        Self { suit, rank }
    }

    #[must_use]
    pub fn code(self) -> String {
        format!("{}{}", self.rank.symbol(), self.suit.symbol())
    }

    #[must_use]
    pub fn accessible_name(self) -> String {
        format!("{} of {}", self.rank.name(), self.suit.name())
    }

    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        let (rank, suit) = match input {
            "9C" => (Rank::Nine, Suit::Clubs),
            "JC" => (Rank::Jack, Suit::Clubs),
            "QC" => (Rank::Queen, Suit::Clubs),
            "KC" => (Rank::King, Suit::Clubs),
            "TC" => (Rank::Ten, Suit::Clubs),
            "AC" => (Rank::Ace, Suit::Clubs),
            "9D" => (Rank::Nine, Suit::Diamonds),
            "JD" => (Rank::Jack, Suit::Diamonds),
            "QD" => (Rank::Queen, Suit::Diamonds),
            "KD" => (Rank::King, Suit::Diamonds),
            "TD" => (Rank::Ten, Suit::Diamonds),
            "AD" => (Rank::Ace, Suit::Diamonds),
            "9H" => (Rank::Nine, Suit::Hearts),
            "JH" => (Rank::Jack, Suit::Hearts),
            "QH" => (Rank::Queen, Suit::Hearts),
            "KH" => (Rank::King, Suit::Hearts),
            "TH" => (Rank::Ten, Suit::Hearts),
            "AH" => (Rank::Ace, Suit::Hearts),
            "9S" => (Rank::Nine, Suit::Spades),
            "JS" => (Rank::Jack, Suit::Spades),
            "QS" => (Rank::Queen, Suit::Spades),
            "KS" => (Rank::King, Suit::Spades),
            "TS" => (Rank::Ten, Suit::Spades),
            "AS" => (Rank::Ace, Suit::Spades),
            _ => return None,
        };
        Some(Self::new(suit, rank))
    }

    #[must_use]
    pub fn ascii_code(self) -> &'static str {
        match (self.rank, self.suit) {
            (Rank::Nine, Suit::Clubs) => "9C",
            (Rank::Jack, Suit::Clubs) => "JC",
            (Rank::Queen, Suit::Clubs) => "QC",
            (Rank::King, Suit::Clubs) => "KC",
            (Rank::Ten, Suit::Clubs) => "TC",
            (Rank::Ace, Suit::Clubs) => "AC",
            (Rank::Nine, Suit::Diamonds) => "9D",
            (Rank::Jack, Suit::Diamonds) => "JD",
            (Rank::Queen, Suit::Diamonds) => "QD",
            (Rank::King, Suit::Diamonds) => "KD",
            (Rank::Ten, Suit::Diamonds) => "TD",
            (Rank::Ace, Suit::Diamonds) => "AD",
            (Rank::Nine, Suit::Hearts) => "9H",
            (Rank::Jack, Suit::Hearts) => "JH",
            (Rank::Queen, Suit::Hearts) => "QH",
            (Rank::King, Suit::Hearts) => "KH",
            (Rank::Ten, Suit::Hearts) => "TH",
            (Rank::Ace, Suit::Hearts) => "AH",
            (Rank::Nine, Suit::Spades) => "9S",
            (Rank::Jack, Suit::Spades) => "JS",
            (Rank::Queen, Suit::Spades) => "QS",
            (Rank::King, Suit::Spades) => "KS",
            (Rank::Ten, Suit::Spades) => "TS",
            (Rank::Ace, Suit::Spades) => "AS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreVisibility {
    Visible,
    Traditional,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchSettings {
    pub score_visibility: ScoreVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayedCard {
    pub seat: Seat,
    pub card: Card,
    pub marriage: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrickSummary {
    pub cards: [PlayedCard; 2],
    pub winner: Seat,
    pub points: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DealEndReason {
    Declared,
    IncorrectDeclaration,
    ClosedStockFailed,
    LastTrick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DealResult {
    pub winner: Seat,
    pub game_points: u8,
    pub reason: DealEndReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DealState {
    pub hands: [Vec<Card>; 2],
    pub talon: VecDeque<Card>,
    pub trump_card: Option<Card>,
    pub trump: Suit,
    pub leader: Seat,
    pub trick: Vec<PlayedCard>,
    pub card_points: [u16; 2],
    pub tricks_won: [u8; 2],
    pub pending_marriages: [u16; 2],
    pub closed_by: Option<Seat>,
    pub public_known_hands: [Vec<Card>; 2],
    pub played_cards: Vec<Card>,
    pub last_trick: Option<TrickSummary>,
    pub result: Option<DealResult>,
}

impl DealState {
    fn deal(seed: u64, dealer: Seat) -> Self {
        let mut deck = full_deck();
        deck.shuffle(&mut ChaCha8Rng::seed_from_u64(seed));
        let mut deck = deck.into_iter();
        let non_dealer = dealer.other();
        let mut hands = [Vec::with_capacity(6), Vec::with_capacity(6)];

        for _ in 0..3 {
            hands[non_dealer.index()].push(deck.next().expect("24-card deck"));
        }
        for _ in 0..3 {
            hands[dealer.index()].push(deck.next().expect("24-card deck"));
        }
        for _ in 0..3 {
            hands[non_dealer.index()].push(deck.next().expect("24-card deck"));
        }
        for _ in 0..3 {
            hands[dealer.index()].push(deck.next().expect("24-card deck"));
        }

        let trump_card = deck.next().expect("trump card remains after deal");
        Self {
            hands,
            talon: deck.collect(),
            trump_card: Some(trump_card),
            trump: trump_card.suit,
            leader: non_dealer,
            trick: Vec::with_capacity(2),
            card_points: [0, 0],
            tricks_won: [0, 0],
            pending_marriages: [0, 0],
            closed_by: None,
            public_known_hands: [Vec::new(), Vec::new()],
            played_cards: Vec::with_capacity(24),
            last_trick: None,
            result: None,
        }
    }

    #[must_use]
    pub fn active_player(&self) -> Seat {
        self.trick
            .first()
            .map_or(self.leader, |lead| lead.seat.other())
    }

    #[must_use]
    pub const fn is_strict(&self) -> bool {
        self.closed_by.is_some() || self.trump_card.is_none()
    }

    #[must_use]
    pub fn stock_count(&self) -> usize {
        self.talon.len() + usize::from(self.trump_card.is_some())
    }

    #[must_use]
    pub fn marriage_value(&self, card: Card) -> Option<u16> {
        card.rank
            .marriage_partner()
            .map(|_| if card.suit == self.trump { 40 } else { 20 })
    }

    #[must_use]
    pub fn can_announce_marriage(&self, seat: Seat, card: Card) -> bool {
        if self.is_strict() || !self.trick.is_empty() || self.leader != seat {
            return false;
        }
        let Some(partner) = card.rank.marriage_partner() else {
            return false;
        };
        self.hands[seat.index()].contains(&Card::new(card.suit, partner))
    }

    #[must_use]
    pub fn can_exchange_trump(&self, seat: Seat) -> bool {
        self.result.is_none()
            && self.closed_by.is_none()
            && !self.talon.is_empty()
            && self.trump_card.is_some()
            && self.trick.is_empty()
            && self.leader == seat
            && self.tricks_won[seat.index()] > 0
            && self.hands[seat.index()].contains(&Card::new(self.trump, Rank::Nine))
    }

    #[must_use]
    pub fn can_close_stock(&self, seat: Seat) -> bool {
        self.result.is_none()
            && self.closed_by.is_none()
            && self.trump_card.is_some()
            && !self.talon.is_empty()
            && self.trick.is_empty()
            && self.leader == seat
    }

    #[must_use]
    pub fn legal_cards(&self, seat: Seat) -> Vec<Card> {
        if self.result.is_some() || self.active_player() != seat {
            return Vec::new();
        }
        let hand = &self.hands[seat.index()];
        let Some(lead) = self.trick.first().map(|play| play.card) else {
            return hand.clone();
        };
        if !self.is_strict() {
            return hand.clone();
        }

        let same_suit: Vec<Card> = hand
            .iter()
            .copied()
            .filter(|card| card.suit == lead.suit)
            .collect();
        if !same_suit.is_empty() {
            let higher: Vec<Card> = same_suit
                .iter()
                .copied()
                .filter(|card| card.rank.strength() > lead.rank.strength())
                .collect();
            return if higher.is_empty() { same_suit } else { higher };
        }

        let trumps: Vec<Card> = hand
            .iter()
            .copied()
            .filter(|card| card.suit == self.trump)
            .collect();
        if trumps.is_empty() {
            hand.clone()
        } else {
            trumps
        }
    }

    fn add_marriage(&mut self, seat: Seat, card: Card) -> u16 {
        let value = self.marriage_value(card).expect("validated marriage");
        let index = seat.index();
        if self.tricks_won[index] == 0 {
            self.pending_marriages[index] += value;
        } else {
            self.card_points[index] += value;
        }
        let partner = Card::new(
            card.suit,
            card.rank.marriage_partner().expect("validated marriage"),
        );
        if !self.public_known_hands[index].contains(&partner) {
            self.public_known_hands[index].push(partner);
        }
        value
    }

    fn remove_known_card(&mut self, seat: Seat, card: Card) {
        self.public_known_hands[seat.index()].retain(|known| *known != card);
    }

    fn draw_for(&mut self, seat: Seat) {
        let card = self.talon.pop_front().or_else(|| self.trump_card.take());
        if let Some(card) = card {
            self.hands[seat.index()].push(card);
        }
    }

    fn resolve_trick(&mut self) -> Seat {
        let first = self.trick[0];
        let second = self.trick[1];
        let winner = if card_beats(second.card, first.card, self.trump) {
            second.seat
        } else {
            first.seat
        };
        let trick_points = first.card.rank.points() + second.card.rank.points();
        let winner_index = winner.index();
        self.card_points[winner_index] += trick_points;
        self.tricks_won[winner_index] += 1;
        self.card_points[winner_index] += self.pending_marriages[winner_index];
        self.pending_marriages[winner_index] = 0;
        self.last_trick = Some(TrickSummary {
            cards: [first, second],
            winner,
            points: trick_points,
        });
        self.trick.clear();
        self.leader = winner;

        if self.closed_by.is_none() && self.trump_card.is_some() {
            self.draw_for(winner);
            self.draw_for(winner.other());
        }

        winner
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchState {
    pub schema_version: u8,
    pub settings: MatchSettings,
    pub match_points: [u8; 2],
    pub dealer: Seat,
    pub deal_number: u32,
    pub next_seed: u64,
    pub deal: DealState,
    pub winner: Option<Seat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    Play {
        card: Card,
        announce_marriage: bool,
        declare: bool,
    },
    ExchangeTrump,
    CloseStock,
    Declare,
    NextDeal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transition {
    pub deal_ended: bool,
    pub match_ended: bool,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuleError {
    #[error("the match is already over")]
    MatchOver,
    #[error("the deal is already over")]
    DealOver,
    #[error("that action is not available now")]
    ActionUnavailable,
    #[error("it is not your turn")]
    NotYourTurn,
    #[error("that card is not in your hand")]
    CardNotHeld,
    #[error("that card is not legal now")]
    IllegalCard,
    #[error("that marriage cannot be announced now")]
    InvalidMarriage,
}

impl MatchState {
    #[must_use]
    pub fn new(seed: u64, score_visibility: ScoreVisibility) -> Self {
        let dealer = if seed & 1 == 0 { Seat::One } else { Seat::Two };
        Self {
            schema_version: 1,
            settings: MatchSettings { score_visibility },
            match_points: [0, 0],
            dealer,
            deal_number: 1,
            next_seed: seed.wrapping_add(0x9E37_79B9_7F4A_7C15),
            deal: DealState::deal(seed, dealer),
            winner: None,
        }
    }

    /// Applies one player action to the authoritative match state.
    ///
    /// # Errors
    ///
    /// Returns a [`RuleError`] when the match phase, turn, card, or requested
    /// declaration does not permit the action.
    pub fn apply(&mut self, seat: Seat, action: Action) -> Result<Transition, RuleError> {
        if self.winner.is_some() {
            return Err(RuleError::MatchOver);
        }
        if matches!(action, Action::NextDeal) {
            return self.next_deal();
        }
        if self.deal.result.is_some() {
            return Err(RuleError::DealOver);
        }

        match action {
            Action::Play {
                card,
                announce_marriage,
                declare,
            } => self.play_card(seat, card, announce_marriage, declare)?,
            Action::ExchangeTrump => self.exchange_trump(seat)?,
            Action::CloseStock => self.close_stock(seat)?,
            Action::Declare => {
                if !self.deal.trick.is_empty() || self.deal.leader != seat {
                    return Err(RuleError::ActionUnavailable);
                }
                self.declare(seat);
            }
            Action::NextDeal => unreachable!("handled above"),
        }

        Ok(Transition {
            deal_ended: self.deal.result.is_some(),
            match_ended: self.winner.is_some(),
        })
    }

    fn play_card(
        &mut self,
        seat: Seat,
        card: Card,
        announce_marriage: bool,
        declare: bool,
    ) -> Result<(), RuleError> {
        if self.deal.active_player() != seat {
            return Err(RuleError::NotYourTurn);
        }
        let hand_index = self.deal.hands[seat.index()]
            .iter()
            .position(|held| *held == card)
            .ok_or(RuleError::CardNotHeld)?;
        if !self.deal.legal_cards(seat).contains(&card) {
            return Err(RuleError::IllegalCard);
        }
        if announce_marriage && !self.deal.can_announce_marriage(seat, card) {
            return Err(RuleError::InvalidMarriage);
        }
        if declare && !announce_marriage {
            return Err(RuleError::ActionUnavailable);
        }

        self.deal.hands[seat.index()].remove(hand_index);
        self.deal.remove_known_card(seat, card);
        let marriage = announce_marriage.then(|| self.deal.add_marriage(seat, card));
        self.deal.trick.push(PlayedCard {
            seat,
            card,
            marriage,
        });
        self.deal.played_cards.push(card);

        if declare {
            self.declare(seat);
            return Ok(());
        }

        if self.deal.trick.len() == 2 {
            let winner = self.deal.resolve_trick();
            if self.deal.hands[0].is_empty() && self.deal.hands[1].is_empty() {
                self.finish_exhausted_deal(winner);
            }
        }
        Ok(())
    }

    fn exchange_trump(&mut self, seat: Seat) -> Result<(), RuleError> {
        if !self.deal.can_exchange_trump(seat) {
            return Err(RuleError::ActionUnavailable);
        }
        let nine = Card::new(self.deal.trump, Rank::Nine);
        let position = self.deal.hands[seat.index()]
            .iter()
            .position(|card| *card == nine)
            .expect("validated trump nine");
        let gained = self
            .deal
            .trump_card
            .replace(nine)
            .expect("validated face-up trump");
        self.deal.hands[seat.index()][position] = gained;
        self.deal.remove_known_card(seat, nine);
        if !self.deal.public_known_hands[seat.index()].contains(&gained) {
            self.deal.public_known_hands[seat.index()].push(gained);
        }
        Ok(())
    }

    fn close_stock(&mut self, seat: Seat) -> Result<(), RuleError> {
        if !self.deal.can_close_stock(seat) {
            return Err(RuleError::ActionUnavailable);
        }
        self.deal.closed_by = Some(seat);
        Ok(())
    }

    fn declare(&mut self, seat: Seat) {
        let index = seat.index();
        if self.deal.card_points[index] >= 66 {
            let points = game_points_for(&self.deal, seat);
            self.award_deal(seat, points, DealEndReason::Declared);
        } else {
            let winner = seat.other();
            let points = if self.deal.tricks_won[winner.index()] == 0 {
                3
            } else {
                2
            };
            self.award_deal(winner, points, DealEndReason::IncorrectDeclaration);
        }
    }

    fn finish_exhausted_deal(&mut self, last_winner: Seat) {
        if let Some(closer) = self.deal.closed_by {
            if self.deal.card_points[closer.index()] >= 66 {
                let points = game_points_for(&self.deal, closer);
                self.award_deal(closer, points, DealEndReason::Declared);
            } else {
                let winner = closer.other();
                let points = if self.deal.tricks_won[winner.index()] == 0 {
                    3
                } else {
                    2
                };
                self.award_deal(winner, points, DealEndReason::ClosedStockFailed);
            }
            return;
        }

        self.deal.card_points[last_winner.index()] += 10;
        let winner = if self.deal.card_points[last_winner.index()] >= 66 {
            last_winner
        } else {
            last_winner.other()
        };
        let points = game_points_for(&self.deal, winner);
        self.award_deal(winner, points, DealEndReason::LastTrick);
    }

    fn award_deal(&mut self, winner: Seat, game_points: u8, reason: DealEndReason) {
        self.deal.result = Some(DealResult {
            winner,
            game_points,
            reason,
        });
        self.match_points[winner.index()] =
            self.match_points[winner.index()].saturating_add(game_points);
        if self.match_points[winner.index()] >= MATCH_TARGET {
            self.winner = Some(winner);
        }
    }

    fn next_deal(&mut self) -> Result<Transition, RuleError> {
        if self.deal.result.is_none() {
            return Err(RuleError::ActionUnavailable);
        }
        self.dealer = self.dealer.other();
        self.deal_number += 1;
        self.deal = DealState::deal(self.next_seed, self.dealer);
        self.next_seed = self.next_seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        Ok(Transition {
            deal_ended: false,
            match_ended: false,
        })
    }
}

#[must_use]
pub fn full_deck() -> Vec<Card> {
    Suit::ALL
        .into_iter()
        .flat_map(|suit| Rank::ALL.into_iter().map(move |rank| Card::new(suit, rank)))
        .collect()
}

#[must_use]
pub fn card_beats(challenger: Card, current: Card, trump: Suit) -> bool {
    if challenger.suit == current.suit {
        challenger.rank.strength() > current.rank.strength()
    } else {
        challenger.suit == trump && current.suit != trump
    }
}

#[must_use]
pub fn game_points_for(deal: &DealState, winner: Seat) -> u8 {
    let loser = winner.other();
    if deal.tricks_won[loser.index()] == 0 {
        3
    } else if deal.card_points[loser.index()] <= 32 {
        2
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(code: &str) -> Card {
        Card::parse(code).expect("valid test card")
    }

    #[test]
    fn deck_has_24_unique_cards_and_120_points() {
        let deck = full_deck();
        let mut unique = deck.clone();
        unique.sort_by_key(|card| (card.suit, card.rank));
        unique.dedup();
        assert_eq!(deck.len(), 24);
        assert_eq!(unique.len(), 24);
        assert_eq!(deck.iter().map(|card| card.rank.points()).sum::<u16>(), 120);
    }

    #[test]
    fn deal_has_six_cards_per_player_and_twelve_in_stock() {
        let state = MatchState::new(42, ScoreVisibility::Visible);
        assert_eq!(state.deal.hands[0].len(), 6);
        assert_eq!(state.deal.hands[1].len(), 6);
        assert_eq!(state.deal.stock_count(), 12);
        assert_eq!(state.deal.leader, state.dealer.other());
    }

    #[test]
    fn ten_is_second_only_to_ace() {
        assert!(card_beats(card("TC"), card("KC"), Suit::Hearts));
        assert!(card_beats(card("AC"), card("TC"), Suit::Hearts));
        assert!(!card_beats(card("KC"), card("TC"), Suit::Hearts));
    }

    #[test]
    fn trump_beats_an_off_suit_card() {
        assert!(card_beats(card("9H"), card("AS"), Suit::Hearts));
        assert!(!card_beats(card("AS"), card("9H"), Suit::Hearts));
    }

    #[test]
    fn open_stock_allows_any_following_card() {
        let mut state = MatchState::new(9, ScoreVisibility::Visible);
        let leader = state.deal.leader;
        let lead = state.deal.hands[leader.index()][0];
        state
            .apply(
                leader,
                Action::Play {
                    card: lead,
                    announce_marriage: false,
                    declare: false,
                },
            )
            .unwrap();
        assert_eq!(state.deal.legal_cards(leader.other()).len(), 6);
    }

    #[test]
    fn strict_play_requires_heading_the_led_suit_when_possible() {
        let mut state = MatchState::new(1, ScoreVisibility::Visible);
        state.deal.closed_by = Some(Seat::One);
        state.deal.leader = Seat::One;
        state.deal.hands = [vec![card("QC")], vec![card("9C"), card("AC"), card("AH")]];
        state.deal.trick = vec![PlayedCard {
            seat: Seat::One,
            card: card("QC"),
            marriage: None,
        }];
        assert_eq!(state.deal.legal_cards(Seat::Two), vec![card("AC")]);
    }

    #[test]
    fn strict_play_requires_trump_when_void() {
        let mut state = MatchState::new(1, ScoreVisibility::Visible);
        state.deal.closed_by = Some(Seat::One);
        state.deal.trump = Suit::Hearts;
        state.deal.leader = Seat::One;
        state.deal.hands[1] = vec![card("9H"), card("AS")];
        state.deal.trick = vec![PlayedCard {
            seat: Seat::One,
            card: card("QC"),
            marriage: None,
        }];
        assert_eq!(state.deal.legal_cards(Seat::Two), vec![card("9H")]);
    }

    #[test]
    fn marriage_waits_for_first_trick() {
        let mut state = MatchState::new(1, ScoreVisibility::Visible);
        state.deal.trump = Suit::Spades;
        state.deal.leader = Seat::One;
        state.deal.hands[0] = vec![card("KH"), card("QH")];
        state.deal.hands[1] = vec![card("9C")];
        state.deal.talon.clear();
        state.deal.trump_card = None;
        state.deal.closed_by = None;
        // Temporarily open the stock to exercise a marriage.
        state.deal.trump_card = Some(card("9S"));
        state.deal.talon.push_back(card("JS"));

        state
            .apply(
                Seat::One,
                Action::Play {
                    card: card("KH"),
                    announce_marriage: true,
                    declare: false,
                },
            )
            .unwrap();
        assert_eq!(state.deal.card_points[0], 0);
        assert_eq!(state.deal.pending_marriages[0], 20);
    }

    #[test]
    fn losing_the_marriage_trick_keeps_the_bonus_pending_and_passes_the_lead() {
        let mut state = MatchState::new(21, ScoreVisibility::Visible);
        state.deal.trump = Suit::Diamonds;
        state.deal.leader = Seat::One;
        state.deal.hands[0] = vec![card("KD"), card("QD")];
        state.deal.hands[1] = vec![card("AD")];
        state.deal.talon = VecDeque::from([card("9C"), card("JC")]);
        state.deal.trump_card = Some(card("9D"));

        state
            .apply(
                Seat::One,
                Action::Play {
                    card: card("KD"),
                    announce_marriage: true,
                    declare: false,
                },
            )
            .unwrap();
        state
            .apply(
                Seat::Two,
                Action::Play {
                    card: card("AD"),
                    announce_marriage: false,
                    declare: false,
                },
            )
            .unwrap();

        assert_eq!(state.deal.card_points, [0, 15]);
        assert_eq!(state.deal.pending_marriages, [40, 0]);
        assert_eq!(state.deal.leader, Seat::Two);
        assert_eq!(state.deal.active_player(), Seat::Two);
        assert_eq!(
            state.deal.last_trick.as_ref().map(|trick| trick.winner),
            Some(Seat::Two)
        );
    }

    #[test]
    fn an_incorrect_declaration_awards_the_opponent() {
        let mut state = MatchState::new(3, ScoreVisibility::Traditional);
        state.deal.tricks_won[1] = 1;
        state.deal.leader = Seat::One;
        state.apply(Seat::One, Action::Declare).unwrap();
        assert_eq!(
            state.deal.result,
            Some(DealResult {
                winner: Seat::Two,
                game_points: 2,
                reason: DealEndReason::IncorrectDeclaration,
            })
        );
    }

    #[test]
    fn declaration_is_only_available_to_the_player_on_lead() {
        let mut state = MatchState::new(8, ScoreVisibility::Visible);
        let leader = state.deal.leader;
        assert_eq!(
            state.apply(leader.other(), Action::Declare),
            Err(RuleError::ActionUnavailable)
        );

        let card = state.deal.hands[leader.index()][0];
        state
            .apply(
                leader,
                Action::Play {
                    card,
                    announce_marriage: false,
                    declare: false,
                },
            )
            .unwrap();
        assert_eq!(
            state.apply(leader.other(), Action::Declare),
            Err(RuleError::ActionUnavailable)
        );
    }

    #[test]
    fn exchange_swaps_the_trump_nine_and_reveals_the_gained_card() {
        let mut state = MatchState::new(2, ScoreVisibility::Visible);
        let seat = Seat::One;
        state.deal.leader = seat;
        state.deal.tricks_won[seat.index()] = 1;
        state.deal.trump = Suit::Hearts;
        state.deal.trump_card = Some(card("AH"));
        state.deal.talon = VecDeque::from([card("9C")]);
        state.deal.hands[seat.index()] = vec![card("9H"), card("QS")];

        state.apply(seat, Action::ExchangeTrump).unwrap();
        assert_eq!(state.deal.trump_card, Some(card("9H")));
        assert!(state.deal.hands[seat.index()].contains(&card("AH")));
        assert!(state.deal.public_known_hands[seat.index()].contains(&card("AH")));
    }

    #[test]
    fn closed_stock_stops_drawing_and_enables_strict_play() {
        let mut state = MatchState::new(12, ScoreVisibility::Visible);
        let leader = state.deal.leader;
        let stock_before = state.deal.stock_count();
        state.apply(leader, Action::CloseStock).unwrap();
        assert!(state.deal.is_strict());

        let lead = state.deal.legal_cards(leader)[0];
        state
            .apply(
                leader,
                Action::Play {
                    card: lead,
                    announce_marriage: false,
                    declare: false,
                },
            )
            .unwrap();
        let follower = leader.other();
        let follow = state.deal.legal_cards(follower)[0];
        state
            .apply(
                follower,
                Action::Play {
                    card: follow,
                    announce_marriage: false,
                    declare: false,
                },
            )
            .unwrap();
        assert_eq!(state.deal.stock_count(), stock_before);
        assert_eq!(state.deal.hands[0].len(), 5);
        assert_eq!(state.deal.hands[1].len(), 5);
    }

    #[test]
    fn hundreds_of_seeded_matches_finish_without_losing_cards_or_deadlocking() {
        for seed in 0..200_u64 {
            let mut state = MatchState::new(seed, ScoreVisibility::Visible);
            let mut actions = 0_u32;
            while state.winner.is_none() {
                assert!(actions < 1_000, "match {seed} deadlocked");
                assert_card_conservation(&state.deal);
                if state.deal.result.is_some() {
                    state
                        .apply(Seat::One, Action::NextDeal)
                        .expect("start next seeded deal");
                    actions += 1;
                    continue;
                }

                let seat = state.deal.active_player();
                if state.deal.trick.is_empty()
                    && state.deal.leader == seat
                    && state.deal.card_points[seat.index()] >= 66
                {
                    state.apply(seat, Action::Declare).unwrap();
                    actions += 1;
                    continue;
                }
                if actions.is_multiple_of(29) && state.deal.can_exchange_trump(seat) {
                    state.apply(seat, Action::ExchangeTrump).unwrap();
                    actions += 1;
                    continue;
                }
                if actions.is_multiple_of(37) && state.deal.can_close_stock(seat) {
                    state.apply(seat, Action::CloseStock).unwrap();
                    actions += 1;
                    continue;
                }
                let legal = state.deal.legal_cards(seat);
                assert!(!legal.is_empty(), "active deal {seed} has no legal play");
                let card = legal[usize::try_from(actions).unwrap_or_default() % legal.len()];
                let announce = state.deal.can_announce_marriage(seat, card);
                let marriage_value = state.deal.marriage_value(card).unwrap_or_default();
                let declare = announce
                    && state.deal.tricks_won[seat.index()] > 0
                    && state.deal.card_points[seat.index()] + marriage_value >= 66;
                state
                    .apply(
                        seat,
                        Action::Play {
                            card,
                            announce_marriage: announce,
                            declare,
                        },
                    )
                    .unwrap();
                actions += 1;
            }
            assert!(state.match_points[state.winner.unwrap().index()] >= MATCH_TARGET);
        }
    }

    fn assert_card_conservation(deal: &DealState) {
        let mut cards = Vec::new();
        cards.extend(deal.hands[0].iter().copied());
        cards.extend(deal.hands[1].iter().copied());
        cards.extend(deal.talon.iter().copied());
        cards.extend(deal.trump_card);
        cards.extend(deal.played_cards.iter().copied());
        assert_eq!(cards.len(), 24);
        cards.sort_by_key(|card| (card.suit, card.rank));
        cards.dedup();
        assert_eq!(cards.len(), 24);
        for seat in [Seat::One, Seat::Two] {
            assert!(
                deal.public_known_hands[seat.index()]
                    .iter()
                    .all(|card| deal.hands[seat.index()].contains(card))
            );
        }
    }
}
