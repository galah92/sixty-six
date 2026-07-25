use rand::SeedableRng;
use rand::seq::SliceRandom;
use rand_chacha::ChaCha8Rng;

use crate::game::{Action, Card, MatchState, Rank, Seat, Suit};

/// Everything the computer is allowed to know.
///
/// Deliberately copying public fields into this type prevents bot strategies
/// from accidentally inspecting the real opponent hand or talon order.
#[derive(Debug, Clone)]
pub struct Observation {
    pub seat: Seat,
    pub hand: Vec<Card>,
    pub trump: Suit,
    pub face_up_trump: Option<Card>,
    pub stock_count: usize,
    pub closed_by: Option<Seat>,
    pub leader: Seat,
    pub trick: Vec<(Seat, Card)>,
    pub card_points: [u16; 2],
    pub tricks_won: [u8; 2],
    pub pending_marriages: [u16; 2],
    pub public_known_opponent_cards: Vec<Card>,
    pub played_cards: Vec<Card>,
}

impl Observation {
    #[must_use]
    pub fn for_player(state: &MatchState, seat: Seat) -> Self {
        Self {
            seat,
            hand: state.deal.hands[seat.index()].clone(),
            trump: state.deal.trump,
            face_up_trump: state.deal.trump_card,
            stock_count: state.deal.stock_count(),
            closed_by: state.deal.closed_by,
            leader: state.deal.leader,
            trick: state
                .deal
                .trick
                .iter()
                .map(|play| (play.seat, play.card))
                .collect(),
            card_points: state.deal.card_points,
            tricks_won: state.deal.tricks_won,
            pending_marriages: state.deal.pending_marriages,
            public_known_opponent_cards: state.deal.public_known_hands[seat.other().index()]
                .clone(),
            played_cards: state.deal.played_cards.clone(),
        }
    }

    #[must_use]
    pub fn is_strict(&self) -> bool {
        self.closed_by.is_some() || self.face_up_trump.is_none()
    }

    #[must_use]
    pub fn is_leading(&self) -> bool {
        self.trick.is_empty() && self.leader == self.seat
    }

    #[must_use]
    pub fn legal_cards(&self) -> Vec<Card> {
        let Some((_, lead)) = self.trick.first().copied() else {
            return self.hand.clone();
        };
        if !self.is_strict() {
            return self.hand.clone();
        }

        let same_suit: Vec<Card> = self
            .hand
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
        let trumps: Vec<Card> = self
            .hand
            .iter()
            .copied()
            .filter(|card| card.suit == self.trump)
            .collect();
        if trumps.is_empty() {
            self.hand.clone()
        } else {
            trumps
        }
    }

    #[must_use]
    pub fn can_announce(&self, card: Card) -> bool {
        if !self.is_leading() || self.is_strict() {
            return false;
        }
        let Some(partner) = card.rank.marriage_partner() else {
            return false;
        };
        self.hand.contains(&Card::new(card.suit, partner))
    }
}

#[must_use]
pub fn choose_action(observation: &Observation, entropy: u64) -> Action {
    let seat = observation.seat;
    if observation.card_points[seat.index()] >= 66 {
        return Action::Declare;
    }

    if should_exchange(observation) {
        return Action::ExchangeTrump;
    }
    if should_close(observation) {
        return Action::CloseStock;
    }

    let mut rng = ChaCha8Rng::seed_from_u64(entropy);
    let legal = observation.legal_cards();
    let card = if observation.trick.is_empty() {
        choose_lead(observation, &legal, &mut rng)
    } else {
        choose_follow(observation, &legal, &mut rng)
    };
    let announce_marriage = observation.can_announce(card);
    let marriage_points = if announce_marriage {
        if card.suit == observation.trump {
            40
        } else {
            20
        }
    } else {
        0
    };
    let marriage_is_live = observation.tricks_won[seat.index()] > 0;
    let declare = announce_marriage
        && marriage_is_live
        && observation.card_points[seat.index()] + marriage_points >= 66;

    Action::Play {
        card,
        announce_marriage,
        declare,
    }
}

fn should_exchange(observation: &Observation) -> bool {
    observation.is_leading()
        && observation.closed_by.is_none()
        && observation.stock_count > 1
        && observation.tricks_won[observation.seat.index()] > 0
        && observation.face_up_trump.is_some()
        && observation
            .hand
            .contains(&Card::new(observation.trump, Rank::Nine))
}

fn should_close(observation: &Observation) -> bool {
    if !observation.is_leading() || observation.closed_by.is_some() || observation.stock_count <= 1
    {
        return false;
    }
    let seat = observation.seat.index();
    let points = observation.card_points[seat];
    let hand_points: u16 = observation.hand.iter().map(|card| card.rank.points()).sum();
    let strong_trumps = observation
        .hand
        .iter()
        .filter(|card| {
            card.suit == observation.trump
                && matches!(card.rank, Rank::Ace | Rank::Ten | Rank::King)
        })
        .count();
    let aces = observation
        .hand
        .iter()
        .filter(|card| card.rank == Rank::Ace)
        .count();
    points >= 60
        || (points >= 50 && points + hand_points >= 72 && strong_trumps >= 2)
        || (points >= 54 && aces >= 2)
}

fn choose_lead(observation: &Observation, legal: &[Card], rng: &mut ChaCha8Rng) -> Card {
    let mut marriages: Vec<Card> = legal
        .iter()
        .copied()
        .filter(|card| observation.can_announce(*card))
        .collect();
    if !marriages.is_empty() {
        marriages.sort_by_key(|card| {
            (
                card.suit != observation.trump,
                card.rank != Rank::Queen,
                card.suit,
            )
        });
        return marriages[0];
    }

    let mut cards = legal.to_vec();
    cards.shuffle(rng);
    if observation.is_strict() {
        cards.sort_by_key(|card| {
            let known_opponent_has_suit = observation
                .public_known_opponent_cards
                .iter()
                .any(|known| known.suit == card.suit);
            (
                !known_opponent_has_suit,
                card.suit != observation.trump,
                std::cmp::Reverse(card.rank.strength()),
            )
        });
        return cards[0];
    }

    cards.sort_by_key(|card| {
        (
            card.suit == observation.trump,
            card.rank.points(),
            card.rank.strength(),
        )
    });
    cards[0]
}

fn choose_follow(observation: &Observation, legal: &[Card], rng: &mut ChaCha8Rng) -> Card {
    let lead = observation.trick[0].1;
    let mut winning: Vec<Card> = legal
        .iter()
        .copied()
        .filter(|card| crate::game::card_beats(*card, lead, observation.trump))
        .collect();
    if !winning.is_empty() {
        winning.shuffle(rng);
        winning.sort_by_key(|card| {
            (
                card.suit == observation.trump && lead.suit != observation.trump,
                card.rank.strength(),
                card.rank.points(),
            )
        });
        let trick_value = lead.rank.points();
        if observation.is_strict()
            || trick_value >= 4
            || winning[0].rank.points() <= trick_value.saturating_add(2)
        {
            return winning[0];
        }
    }

    let mut discards = legal.to_vec();
    discards.shuffle(rng);
    discards.sort_by_key(|card| {
        (
            card.suit == observation.trump,
            card.rank.points(),
            card.rank.strength(),
        )
    });
    discards[0]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ScoreVisibility;

    #[test]
    fn observation_does_not_contain_hidden_cards() {
        let state = MatchState::new(123, ScoreVisibility::Visible);
        let observation = Observation::for_player(&state, Seat::Two);
        assert_eq!(observation.hand, state.deal.hands[1]);
        assert!(
            !observation
                .hand
                .iter()
                .any(|card| state.deal.hands[0].contains(card))
        );
        assert!(observation.played_cards.is_empty());
        assert!(observation.public_known_opponent_cards.is_empty());
    }

    #[test]
    fn bot_always_chooses_a_legal_initial_play() {
        for seed in 0..100 {
            let state = MatchState::new(seed, ScoreVisibility::Visible);
            let seat = state.deal.leader;
            let observation = Observation::for_player(&state, seat);
            let action = choose_action(&observation, seed);
            match action {
                Action::Play { card, .. } => assert!(state.deal.legal_cards(seat).contains(&card)),
                Action::ExchangeTrump | Action::CloseStock | Action::Declare | Action::NextDeal => {
                    panic!("unexpected initial bot action: {action:?}")
                }
            }
        }
    }
}
