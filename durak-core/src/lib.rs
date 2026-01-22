//! Durak core engine primitives.
//!
//! This crate focuses on *state transitions* and a minimal
//! determinization + rollout-based evaluator.

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum Rank {
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
    Ace = 14,
}

/// A card with visibility - who knows what this card is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Card {
    /// Everyone knows this card (e.g., played cards, trump card).
    Public { suit: Suit, rank: Rank },
    /// Only the holder knows this card (e.g., cards in hand before playing).
    Private { suit: Suit, rank: Rank },
    /// No one knows this card (e.g., opponent cards in manual mode).
    Unknown,
}

impl Card {
    pub fn public(suit: Suit, rank: Rank) -> Self {
        Card::Public { suit, rank }
    }

    pub fn private(suit: Suit, rank: Rank) -> Self {
        Card::Private { suit, rank }
    }

    pub fn suit(&self) -> Suit {
        match self {
            Card::Public { suit, .. } | Card::Private { suit, .. } => *suit,
            Card::Unknown => panic!("cannot get suit of unknown card"),
        }
    }

    pub fn rank(&self) -> Rank {
        match self {
            Card::Public { rank, .. } | Card::Private { rank, .. } => *rank,
            Card::Unknown => panic!("cannot get rank of unknown card"),
        }
    }

    pub fn is_public(&self) -> bool {
        matches!(self, Card::Public { .. })
    }

    pub fn is_private(&self) -> bool {
        matches!(self, Card::Private { .. })
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, Card::Unknown)
    }

    pub fn beats(&self, other: &Card, trump: Suit) -> bool {
        if self.suit() == other.suit() {
            return self.rank() > other.rank();
        }
        self.suit() == trump && other.suit() != trump
    }

    /// Check if this card matches another card (same suit and rank, ignoring visibility).
    pub fn matches(&self, other: &Card) -> bool {
        match (self, other) {
            (Card::Unknown, _) | (_, Card::Unknown) => false,
            _ => self.suit() == other.suit() && self.rank() == other.rank(),
        }
    }

    /// Create a copy as public.
    pub fn as_public(&self) -> Self {
        Card::Public { suit: self.suit(), rank: self.rank() }
    }

    /// Create a copy as private.
    pub fn as_private(&self) -> Self {
        Card::Private { suit: self.suit(), rank: self.rank() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PlayerId {
    #[default]
    P0,
    P1,
    P2,
    P3,
    P4,
    P5,
}

impl PlayerId {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => PlayerId::P0,
            1 => PlayerId::P1,
            2 => PlayerId::P2,
            3 => PlayerId::P3,
            4 => PlayerId::P4,
            _ => PlayerId::P5,
        }
    }

    pub fn other(self) -> PlayerId {
        match self {
            PlayerId::P0 => PlayerId::P1,
            PlayerId::P1 => PlayerId::P0,
            _ => PlayerId::P0,
        }
    }

    pub fn next(self, num_players: usize) -> PlayerId {
        PlayerId::from_index((self as usize + 1) % num_players)
    }

    /// Get next player, skipping players with empty hands (who are out of the game).
    pub fn next_active(self, num_players: usize, hands: &[Vec<Card>]) -> PlayerId {
        let mut next = self.next(num_players);
        let start = next;
        loop {
            if hands[next as usize].len() > 0 {
                return next;
            }
            next = next.next(num_players);
            if next == start {
                // All other players are out, return self
                return self;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Pile {
    pub attack: Card,
    pub defense: Option<Card>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Attacker may add a card or end attack.
    Attacking,
    /// Defender must defend an open pile or take.
    Defending,
    /// Defender decided to take; attacker may throw extra cards.
    Throwing,
}

/// Player type for AI behavior configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlayerType {
    /// Human player - manual input, optional MCTS hints.
    Human,
    /// Random AI - picks a random legal action.
    Random,
    /// MCTS AI - runs determinized MCTS, picks best action.
    MCTS,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GameConfig {
    /// Number of cards in the deck (32, 36, 40, 44, 48, or 52)
    pub deck_size: usize,
    /// Number of players (2-6)
    pub num_players: usize,
    /// Trump reflecting: if true, a trump can be reflected back to attacker
    pub trump_reflecting: bool,
    /// Reflecting: if true, defender can reflect an attack with a card of same rank
    pub reflecting: bool,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            deck_size: 36,
            num_players: 2,
            trump_reflecting: false,
            reflecting: false,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("illegal move: {0}")]
    IllegalMove(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Attacker plays an attacking card.
    Attack { card: Card },
    /// Attacker ends attack (like `passattack` in github-durak).
    PassAttack,

    /// Defender defends a pile.
    Defend { pile_index: usize, card: Card },
    /// Defender takes the table (like `take`).
    Take,

    /// Attacker throws an extra card after a take, or None to finish throwing.
    Throw { card: Option<Card> },

    /// Defender reflects with a card of the same rank (requires `config.reflecting`).
    /// The defender plays the card and the attack is redirected to the next player.
    Reflect { card: Card },

    /// Defender shows a trump card of the same rank without playing it (requires `config.trump_reflecting`).
    /// The card stays in hand, but the attack is redirected. Can only be used once per trump per turn.
    ReflectTrump { card: Card },
}

/// Game state with full card tracking.
///
/// Cards have visibility (Public/Private/Unknown) to track who knows what.
/// - Computer mode: All cards are known to the system, private to holders until played.
/// - Manual mode: Only your cards are known (private), opponents have unknown cards.
///
/// Call `determinize(perspective, rng)` before MCTS to create a state where
/// all cards are known from the perspective player's viewpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameState {
    /// Trump suit.
    pub trump: Suit,

    /// Primary attacker (first in seat order among attackers).
    pub attacker: PlayerId,
    pub defender: PlayerId,
    pub phase: Phase,

    /// List of players who can attack (all active players except defender).
    /// The first player is the primary attacker.
    #[serde(default)]
    pub attackers: Vec<PlayerId>,

    /// Index into `attackers` for whose turn it is to attack/throw.
    #[serde(default)]
    pub current_attacker_idx: usize,

    /// The attacker who last played a card (used to detect when all attackers have passed).
    #[serde(default)]
    pub last_played_attacker: PlayerId,

    /// When throwing starts, the attacker index where we started (to detect completion).
    #[serde(default)]
    pub throw_start_idx: usize,

    /// Player hands. Each card has visibility indicating who knows it.
    pub hands: Vec<Vec<Card>>,

    /// The stock pile (draw pile). Cards are drawn from the end (pop).
    /// In computer mode: bottom card is public (trump), rest are private.
    /// In manual mode: bottom card is public (trump), rest are unknown.
    pub stock: Vec<Card>,

    pub table: Vec<Pile>,
    pub discard: Vec<Card>,

    /// Trump cards that have been shown via ReflectTrump this turn (cannot be used again).
    #[serde(default)]
    pub reflected_trumps: Vec<Card>,

    /// Game configuration.
    pub config: GameConfig,
}

impl GameState {
    /// Build the list of attackers for a new trick.
    /// Attackers are all active players excluding the defender.
    /// The primary attacker (main_attacker) should be first in the list.
    fn build_attackers(&self, main_attacker: PlayerId) -> Vec<PlayerId> {
        let num_players = self.num_players();
        let mut attackers = Vec::with_capacity(num_players - 1);

        // Start from main_attacker and go around, adding players who are still in the game
        let mut pid = main_attacker;
        for _ in 0..num_players {
            if pid != self.defender && self.is_player_active(pid) {
                attackers.push(pid);
            }
            pid = pid.next(num_players);
        }

        attackers
    }

    /// Check if a player is still in the game (has cards or can draw).
    fn is_player_active(&self, pid: PlayerId) -> bool {
        self.hand_size(pid) > 0 || !self.stock.is_empty()
    }

    /// Find the next active player after the given player.
    fn next_active_player(&self, from: PlayerId) -> PlayerId {
        let num_players = self.num_players();
        let mut pid = from.next(num_players);
        let start = pid;
        loop {
            if self.is_player_active(pid) {
                return pid;
            }
            pid = pid.next(num_players);
            if pid == start {
                return from; // No active player found, return original
            }
        }
    }

    /// Start a new trick with the given main attacker.
    fn new_trick(&mut self, main_attacker: PlayerId) {
        // If main_attacker is out of the game, find next active player
        let main_attacker = if self.is_player_active(main_attacker) {
            main_attacker
        } else {
            self.next_active_player(main_attacker)
        };

        // Find the defender (next active player after main_attacker)
        let defender = self.next_active_player(main_attacker);

        self.attacker = main_attacker;
        self.defender = defender;
        self.attackers = self.build_attackers(main_attacker);
        self.current_attacker_idx = 0;
        self.last_played_attacker = main_attacker;
        self.throw_start_idx = 0;
        self.phase = Phase::Attacking;
        self.reflected_trumps.clear();
        self.table.clear();
    }

    /// Create a new computer game with shuffled deck.
    /// All cards are known to the system but private to their holders until played.
    pub fn new_computer_game(seed: u64, config: GameConfig) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);

        // Create a deck of private cards and shuffle it
        let mut deck = full_deck(config.deck_size);
        for i in (1..deck.len()).rev() {
            let j = rng.gen_range(0..=i);
            deck.swap(i, j);
        }

        // Trump card is at the bottom (last card), public
        deck[0] = deck[0].as_public();
        let trump = deck[0].suit();

        // Deal 6 cards to each player (private)
        // Track who has the lowest trump to determine starting player
        let mut hands: Vec<Vec<Card>> = Vec::with_capacity(config.num_players);
        let mut lowest_trump: Option<(usize, Rank)> = None; // (player_index, rank)

        for player_idx in 0..config.num_players {
            let mut hand: Vec<Card> = deck.drain(deck.len() - 6..).collect();

            // Find lowest trump and make it public (announced at game start)
            if let Some(lowest_trump_idx) = hand.iter()
                .enumerate()
                .filter(|(_, c)| c.suit() == trump)
                .min_by_key(|(_, c)| c.rank() as u8)
                .map(|(i, _)| i)
            {
                let rank = hand[lowest_trump_idx].rank();
                hand[lowest_trump_idx] = hand[lowest_trump_idx].as_public();

                // Track global lowest trump
                if lowest_trump.map_or(true, |(_, r)| rank < r) {
                    lowest_trump = Some((player_idx, rank));
                }
            }

            hands.push(hand);
        }

        // Remaining cards are the stock (private, except trump at bottom)
        let stock: Vec<Card> = deck;

        // Player with lowest trump starts, default to P0 if no one has trumps
        let attacker = PlayerId::from_index(lowest_trump.map_or(0, |(idx, _)| idx));
        let defender = attacker.next(config.num_players);

        // Build attackers list (all players except defender)
        let mut attackers = Vec::with_capacity(config.num_players - 1);
        let mut pid = attacker;
        for _ in 0..config.num_players {
            if pid != defender {
                attackers.push(pid);
            }
            pid = pid.next(config.num_players);
        }

        Self {
            trump,
            attacker,
            defender,
            phase: Phase::Attacking,
            attackers,
            current_attacker_idx: 0,
            last_played_attacker: attacker,
            throw_start_idx: 0,
            hands,
            stock,
            table: vec![],
            discard: vec![],
            reflected_trumps: vec![],
            config,
        }
    }

    /// Create a manual game where only your hand is known.
    /// Your cards are private, opponent cards are unknown.
    pub fn new_manual_game(
        trump_card: Card,
        player_hand: Vec<Card>,
        starting_player: u8,
        opponent_lowest_trumps: Vec<(PlayerId, Option<Rank>)>,
        config: GameConfig,
    ) -> Result<Self, EngineError> {
        if player_hand.len() != 6 {
            return Err(EngineError::IllegalMove(
                "manual start requires exactly 6 cards in your hand".into(),
            ));
        }

        let trump = trump_card.suit();

        // Build P0's hand (private, except lowest trump which is public)
        let mut p0_hand: Vec<Card> = player_hand.iter()
            .map(|c| c.as_private())
            .collect();

        // Make lowest trump public
        if let Some(lowest_trump_idx) = p0_hand.iter()
            .enumerate()
            .filter(|(_, c)| c.suit() == trump)
            .min_by_key(|(_, c)| c.rank() as u8)
            .map(|(i, _)| i)
        {
            p0_hand[lowest_trump_idx] = p0_hand[lowest_trump_idx].as_public();
        }

        let mut hands: Vec<Vec<Card>> = Vec::with_capacity(config.num_players);
        hands.push(p0_hand);

        // Opponents: declared lowest trump is public, rest are unknown
        for i in 1..config.num_players {
            let pid = PlayerId::from_index(i);
            let lowest_trump_rank = opponent_lowest_trumps
                .iter()
                .find(|(p, _)| *p == pid)
                .and_then(|(_, rank_opt)| *rank_opt);

            let mut hand = Vec::with_capacity(6);

            // Add the declared lowest trump as public if specified
            if let Some(rank) = lowest_trump_rank {
                hand.push(Card::public(trump, rank));
            }

            // Fill rest with unknown cards
            while hand.len() < 6 {
                hand.push(Card::Unknown);
            }

            hands.push(hand);
        }

        // Stock: trump card at bottom (public), rest unknown
        let stock_size = config.deck_size - config.num_players * 6;
        let mut stock = Vec::with_capacity(stock_size);
        stock.push(trump_card.as_public());
        for _ in 1..stock_size {
            stock.push(Card::Unknown);
        }

        let attacker = PlayerId::from_index(starting_player as usize);
        let defender = attacker.next(config.num_players);

        // Build attackers list (all players except defender)
        let mut attackers = Vec::with_capacity(config.num_players - 1);
        let mut pid = attacker;
        for _ in 0..config.num_players {
            if pid != defender {
                attackers.push(pid);
            }
            pid = pid.next(config.num_players);
        }

        Ok(Self {
            trump,
            attacker,
            defender,
            phase: Phase::Attacking,
            attackers,
            current_attacker_idx: 0,
            last_played_attacker: attacker,
            throw_start_idx: 0,
            hands,
            stock,
            table: vec![],
            discard: vec![],
            reflected_trumps: vec![],
            config,
        })
    }

    pub fn hand_size(&self, pid: PlayerId) -> usize {
        self.hands[pid as usize].len()
    }

    pub fn num_players(&self) -> usize {
        self.hands.len()
    }

    /// Returns the durak (loser) if game is terminal.
    pub fn durak(&self) -> Option<PlayerId> {
        if !self.is_terminal() {
            return None;
        }

        let mut durak = None;
        for i in 0..self.num_players() {
            let pid = PlayerId::from_index(i);
            if self.hand_size(pid) > 0 {
                if durak.is_some() {
                    return None; // Multiple players with cards = tie
                }
                durak = Some(pid);
            }
        }
        durak
    }

    pub fn is_terminal(&self) -> bool {
        if !self.stock.is_empty() {
            return false;
        }
        let players_with_cards = (0..self.num_players())
            .filter(|&i| self.hand_size(PlayerId::from_index(i)) > 0)
            .count();
        players_with_cards <= 1
    }

    pub fn actor_to_move(&self) -> PlayerId {
        match self.phase {
            Phase::Attacking | Phase::Throwing => {
                // Current attacker from the attackers list
                if self.attackers.is_empty() {
                    self.attacker
                } else {
                    self.attackers[self.current_attacker_idx % self.attackers.len()]
                }
            }
            Phase::Defending => self.defender,
        }
    }

    fn ranks_on_table(&self) -> HashSet<Rank> {
        let mut set = HashSet::new();
        for p in &self.table {
            set.insert(p.attack.rank());
            if let Some(d) = p.defense {
                set.insert(d.rank());
            }
        }
        set
    }

    fn open_pile_index(&self) -> Option<usize> {
        self.table.iter().position(|p| p.defense.is_none())
    }

    /// Count undefended piles on the table.
    fn undefended_pile_count(&self) -> usize {
        self.table.iter().filter(|p| p.defense.is_none()).count()
    }

    /// Check if more cards can be thrown/attacked (defender has capacity).
    fn defender_has_capacity(&self) -> bool {
        self.undefended_pile_count() < self.hand_size(self.defender)
    }

    /// Find who would become the new defender if current defender reflects.
    /// Returns None if there's no active player to become defender.
    fn potential_reflect_defender(&self) -> Option<PlayerId> {
        // The new defender is the next active player from the attackers list
        if self.attackers.len() > 1 {
            let candidate = self.attackers[1];
            if self.is_player_active(candidate) {
                return Some(candidate);
            }
        }
        // Fallback: find any active player that's not the current defender
        let num_players = self.num_players();
        let mut pid = self.defender.next(num_players);
        for _ in 0..num_players {
            if pid != self.defender && self.is_player_active(pid) {
                return Some(pid);
            }
            pid = pid.next(num_players);
        }
        None
    }

    /// Create a determinized copy of this state from a player's perspective.
    /// Uses the player's known information (Public + their Private cards).
    /// Unknown cards are shuffled and reassigned randomly.
    pub fn determinize(&self, perspective: PlayerId, rng: &mut impl Rng) -> Self {
        let mut state = self.clone();

        // Collect all known cards (visible to perspective)
        let mut known_cards: HashSet<(Suit, Rank)> = HashSet::new();

        for (hand_idx, hand) in self.hands.iter().enumerate() {
            for card in hand {
                // Card is known if Public, or Private in perspective's hand
                if card.is_public() || (card.is_private() && hand_idx == perspective as usize) {
                    known_cards.insert((card.suit(), card.rank()));
                }
            }
        }

        for card in &self.stock {
            if card.is_public() {
                known_cards.insert((card.suit(), card.rank()));
            }
        }

        for card in &self.discard {
            known_cards.insert((card.suit(), card.rank()));
        }

        for pile in &self.table {
            known_cards.insert((pile.attack.suit(), pile.attack.rank()));
            if let Some(d) = pile.defense {
                known_cards.insert((d.suit(), d.rank()));
            }
        }

        // Build pool of unknown cards (full deck minus known)
        let deck = full_deck(self.config.deck_size);
        let mut unknown_pool: Vec<Card> = deck
            .into_iter()
            .filter(|c| !known_cards.contains(&(c.suit(), c.rank())))
            .map(|c| c.as_public())
            .collect();

        // Shuffle the unknown pool
        for i in (1..unknown_pool.len()).rev() {
            let j = rng.gen_range(0..=i);
            unknown_pool.swap(i, j);
        }

        // Collect positions that need cards from the pool
        let mut unknown_positions: Vec<(usize, usize)> = Vec::new();
        let mut unknown_stock_positions: Vec<usize> = Vec::new();

        for (hand_idx, hand) in state.hands.iter().enumerate() {
            for (card_idx, card) in hand.iter().enumerate() {
                let is_unknown = card.is_unknown() ||
                    (card.is_private() && hand_idx != perspective as usize);
                if is_unknown {
                    unknown_positions.push((hand_idx, card_idx));
                }
            }
        }

        for (stock_idx, card) in state.stock.iter().enumerate() {
            if card.is_unknown() || card.is_private() {
                unknown_stock_positions.push(stock_idx);
            }
        }

        // Reassign cards from the shuffled pool
        let mut pool_idx = 0;
        for (hand_idx, card_idx) in unknown_positions {
            if pool_idx < unknown_pool.len() {
                state.hands[hand_idx][card_idx] = unknown_pool[pool_idx];
                pool_idx += 1;
            }
        }

        for stock_idx in unknown_stock_positions {
            if pool_idx < unknown_pool.len() {
                state.stock[stock_idx] = unknown_pool[pool_idx];
                pool_idx += 1;
            }
        }

        // Make perspective's Private cards Public
        for card in &mut state.hands[perspective as usize] {
            if card.is_private() {
                *card = card.as_public();
            }
        }

        state
    }

    /// Get cards a player can use for actions.
    /// If hand contains Unknown cards, returns all unseen cards (any card that could be there).
    /// Otherwise, returns the known cards (Public/Private).
    fn usable_cards(&self, pid: PlayerId) -> Vec<Card> {
        let hand = &self.hands[pid as usize];

        if !hand.iter().any(|c| c.is_unknown()) {
            // No unknown cards - return known cards
            return hand.clone()
        }

        // Hand contains Unknown cards - return all unseen cards
        let mut known_cards: HashSet<(Suit, Rank)> = HashSet::new();

        // Cards on the table are known
        for pile in &self.table {
            known_cards.insert((pile.attack.suit(), pile.attack.rank()));
            if let Some(d) = pile.defense {
                known_cards.insert((d.suit(), d.rank()));
            }
        }

        // Cards in discard are known
        for card in &self.discard {
            known_cards.insert((card.suit(), card.rank()));
        }

        // Public cards in all hands are known
        for hand in &self.hands {
            for card in hand {
                if card.is_public() || card.is_private() {
                    known_cards.insert((card.suit(), card.rank()));
                }
            }
        }

        // Public cards in stock are known (trump card)
        for card in &self.stock {
            if card.is_public() {
                known_cards.insert((card.suit(), card.rank()));
            }
        }

        // Start with the player's own known cards (Public/Private in their hand)
        let mut usable: Vec<Card> = hand.iter()
            .filter(|c| c.is_public() || c.is_private())
            .cloned()
            .collect();

        // Add all unseen cards (cards not known to be elsewhere)
        usable.extend(
            full_deck(self.config.deck_size)
                .into_iter()
                .filter(|c| !known_cards.contains(&(c.suit(), c.rank())))
                .map(|c| c.as_public())
        );

        usable
    }

    pub fn legal_actions(&self) -> Vec<Action> {
        if self.is_terminal() {
            return vec![];
        }

        let current_attacker = self.actor_to_move();

        match self.phase {
            Phase::Attacking => {
                let mut acts = Vec::new();
                let usable = self.usable_cards(current_attacker);

                if self.table.is_empty() {
                    // First attack - can play any card
                    for c in usable {
                        acts.push(Action::Attack { card: c });
                    }
                } else {
                    // Continuing attack - can pass if all piles defended
                    if self.open_pile_index().is_none() {
                        acts.push(Action::PassAttack);
                    }

                    // Can attack with cards matching ranks on table (if defender has capacity)
                    let ranks = self.ranks_on_table();
                    if self.defender_has_capacity() {
                        for c in usable {
                            if ranks.contains(&c.rank()) {
                                acts.push(Action::Attack { card: c });
                            }
                        }
                    }
                }
                acts
            }
            Phase::Defending => {
                let Some(pile_index) = self.open_pile_index() else {
                    return vec![Action::PassAttack];
                };

                let attack = self.table[pile_index].attack;
                let usable = self.usable_cards(self.defender);

                let mut acts = Vec::new();
                let can_reflect = self.table.iter().all(|p| p.defense.is_none())
                    && self.potential_reflect_defender().is_some();

                for c in usable {
                    if c.beats(&attack, self.trump) {
                        acts.push(Action::Defend { pile_index, card: c });
                    }

                    if can_reflect && self.config.reflecting && c.rank() == attack.rank() {
                        acts.push(Action::Reflect { card: c });
                    }

                    if can_reflect && self.config.trump_reflecting
                        && c.rank() == attack.rank()
                        && c.suit() == self.trump
                        && !self.reflected_trumps.iter().any(|r| r.matches(&c))
                    {
                        acts.push(Action::ReflectTrump { card: c });
                    }
                }
                acts.push(Action::Take);
                acts
            }
            Phase::Throwing => {
                let mut acts = vec![Action::Throw { card: None }];

                let ranks = self.ranks_on_table();
                let usable = self.usable_cards(current_attacker);

                // Can only throw if defender has capacity (undefended piles < defender's hand)
                if self.defender_has_capacity() {
                    for c in usable {
                        if ranks.contains(&c.rank()) {
                            acts.push(Action::Throw { card: Some(c) });
                        }
                    }
                }

                acts
            }
        }
    }

    pub fn apply(&mut self, action: &Action) -> Result<(), EngineError> {
        let current_attacker = self.actor_to_move();

        match (self.phase, action) {
            (Phase::Attacking, Action::Attack { card }) => {
                let card = *card;
                if self.hand_size(self.defender) == 0 {
                    return Err(EngineError::IllegalMove("too many piles".into()));
                }
                if !self.table.is_empty() {
                    let ranks = self.ranks_on_table();
                    if !ranks.contains(&card.rank()) {
                        return Err(EngineError::IllegalMove("attack rank not on table".into()));
                    }
                }

                self.remove_from_hand(current_attacker, &card)?;
                // Card becomes public when played
                self.table.push(Pile { attack: card.as_public(), defense: None });
                // Track who last played an attack card
                self.last_played_attacker = current_attacker;
                self.phase = Phase::Defending;
                Ok(())
            }
            (Phase::Attacking, Action::PassAttack) => {
                if self.open_pile_index().is_some() {
                    return Err(EngineError::IllegalMove(
                        "cannot pass while there are undefended piles".into(),
                    ));
                }

                // Move to next attacker
                if !self.attackers.is_empty() {
                    self.current_attacker_idx = (self.current_attacker_idx + 1) % self.attackers.len();
                    let next_attacker = self.attackers[self.current_attacker_idx];

                    // If we've cycled back to the last player who attacked, everyone has passed
                    if next_attacker != self.last_played_attacker {
                        // More attackers can still play, stay in attacking phase
                        return Ok(());
                    }
                }

                // All attackers have passed - successful defense
                // Discard the table (cards stay public in discard)
                for p in self.table.drain(..) {
                    self.discard.push(p.attack);
                    if let Some(d) = p.defense {
                        self.discard.push(d);
                    }
                }

                // Refill hands in draw order (attackers first, then defender)
                self.refill_hands();

                // Start new trick with defender as main attacker
                self.new_trick(self.defender);
                Ok(())
            }
            (Phase::Defending, Action::Defend { pile_index, card }) => {
                let pile_index = *pile_index;
                let card = *card;
                if pile_index >= self.table.len() {
                    return Err(EngineError::IllegalMove("bad pile index".into()));
                }
                if self.table[pile_index].defense.is_some() {
                    return Err(EngineError::IllegalMove("pile already defended".into()));
                }
                let attack = self.table[pile_index].attack;
                if !card.beats(&attack, self.trump) {
                    return Err(EngineError::IllegalMove(
                        "defense card does not beat attack".into(),
                    ));
                }

                self.remove_from_hand(self.defender, &card)?;
                self.table[pile_index].defense = Some(card.as_public());

                if self.open_pile_index().is_none() {
                    self.phase = Phase::Attacking;
                }

                Ok(())
            }
            (Phase::Defending, Action::Take) => {
                // Remember which attacker starts the throwing phase
                self.throw_start_idx = self.current_attacker_idx;
                self.phase = Phase::Throwing;
                Ok(())
            }
            (Phase::Defending, Action::Reflect { card }) => {
                let card = *card;
                if !self.config.reflecting {
                    return Err(EngineError::IllegalMove("reflecting not enabled".into()));
                }
                if self.table.iter().any(|p| p.defense.is_some()) {
                    return Err(EngineError::IllegalMove(
                        "cannot reflect after defending a pile".into(),
                    ));
                }
                let Some(pile_index) = self.open_pile_index() else {
                    return Err(EngineError::IllegalMove("no open pile to reflect".into()));
                };
                let attack = self.table[pile_index].attack;
                if card.rank() != attack.rank() {
                    return Err(EngineError::IllegalMove(
                        "reflect card must have same rank as attack".into(),
                    ));
                }

                // Find new defender - must be an active player
                let Some(new_defender) = self.potential_reflect_defender() else {
                    return Err(EngineError::IllegalMove(
                        "no active player to become defender".into(),
                    ));
                };

                self.remove_from_hand(self.defender, &card)?;
                self.table.push(Pile { attack: card.as_public(), defense: None });

                // The defender who reflected becomes the new attacker
                let old_defender = self.defender;
                self.attacker = old_defender;
                self.defender = new_defender;

                // Rebuild attackers list
                self.attackers = self.build_attackers(self.attacker);
                self.current_attacker_idx = 0;
                self.last_played_attacker = self.attacker;

                self.phase = Phase::Defending;
                Ok(())
            }
            (Phase::Defending, Action::ReflectTrump { card }) => {
                let card = *card;
                if !self.config.trump_reflecting {
                    return Err(EngineError::IllegalMove("trump reflecting not enabled".into()));
                }
                if self.table.iter().any(|p| p.defense.is_some()) {
                    return Err(EngineError::IllegalMove(
                        "cannot reflect after defending a pile".into(),
                    ));
                }
                let Some(pile_index) = self.open_pile_index() else {
                    return Err(EngineError::IllegalMove("no open pile to reflect".into()));
                };
                let attack = self.table[pile_index].attack;
                if card.rank() != attack.rank() {
                    return Err(EngineError::IllegalMove(
                        "reflect card must have same rank as attack".into(),
                    ));
                }
                if card.suit() != self.trump {
                    return Err(EngineError::IllegalMove(
                        "trump reflect requires a trump card".into(),
                    ));
                }
                if self.reflected_trumps.iter().any(|r| r.matches(&card)) {
                    return Err(EngineError::IllegalMove(
                        "this trump has already been used for reflecting this turn".into(),
                    ));
                }

                // Find new defender - must be an active player
                let Some(new_defender) = self.potential_reflect_defender() else {
                    return Err(EngineError::IllegalMove(
                        "no active player to become defender".into(),
                    ));
                };

                // Card stays in hand but becomes public, marked as used
                self.reflected_trumps.push(card.as_public());

                // Make the card public in hand
                if let Some(c) = self.hands[self.defender as usize].iter_mut()
                    .find(|c| c.matches(&card))
                {
                    *c = c.as_public();
                }

                // The defender who reflected becomes the new attacker
                let old_defender = self.defender;
                self.attacker = old_defender;
                self.defender = new_defender;

                // Rebuild attackers list
                self.attackers = self.build_attackers(self.attacker);
                self.current_attacker_idx = 0;
                self.last_played_attacker = self.attacker;

                self.phase = Phase::Defending;
                Ok(())
            }
            (Phase::Throwing, Action::Throw { card }) => {
                if let Some(c) = card {
                    let ranks = self.ranks_on_table();
                    if !ranks.contains(&c.rank()) {
                        return Err(EngineError::IllegalMove("thrown rank not on table".into()));
                    }
                    if self.hand_size(self.defender) == 0 {
                        return Err(EngineError::IllegalMove("defender has no capacity".into()));
                    }
                    self.remove_from_hand(current_attacker, c)?;
                    self.table.push(Pile { attack: c.as_public(), defense: None });
                    return Ok(());
                }

                // None = done throwing for this attacker, move to next
                if !self.attackers.is_empty() {
                    self.current_attacker_idx = (self.current_attacker_idx + 1) % self.attackers.len();

                    // If we haven't cycled back to start, more attackers can throw
                    if self.current_attacker_idx != self.throw_start_idx {
                        return Ok(());
                    }
                }

                // All attackers done throwing - defender takes everything
                let taken: Vec<Card> = self.table.drain(..)
                    .flat_map(|p| {
                        let mut cards = vec![p.attack];
                        if let Some(d) = p.defense {
                            cards.push(d);
                        }
                        cards
                    })
                    .collect();

                // Cards taken are public (they were on the table)
                self.hands[self.defender as usize].extend(taken);

                // Refill hands in draw order (attackers first, then defender)
                self.refill_hands();

                // Start new trick - main attacker is player after the defender
                let num_players = self.num_players();
                let new_main_attacker = self.defender.next(num_players);
                self.new_trick(new_main_attacker);
                Ok(())
            }
            (_, a) => Err(EngineError::IllegalMove(format!(
                "action not allowed in this phase: {a:?}"
            ))),
        }
    }

    /// Remove a card from a player's hand (matching by suit and rank).
    /// If the card isn't found but the hand has Unknown cards, removes one Unknown
    /// (the opponent "reveals" that their unknown card was this specific card).
    fn remove_from_hand(&mut self, pid: PlayerId, card: &Card) -> Result<(), EngineError> {
        let hand = &mut self.hands[pid as usize];
        // First try exact match
        if let Some(i) = hand.iter().position(|c| c.matches(card)) {
            hand.swap_remove(i);
            return Ok(());
        }
        // If not found, try removing an Unknown card (manual mode: opponent plays a card we didn't know they had)
        if let Some(i) = hand.iter().position(|c| c.is_unknown()) {
            hand.swap_remove(i);
            return Ok(());
        }
        Err(EngineError::IllegalMove(format!("card not in {pid:?}'s hand")))
    }

    /// Refill hands after a round ends by drawing from stock.
    /// Draw order: attackers first (in order), then defender.
    fn refill_hands(&mut self) {
        // Build draw order: attackers first, then defender
        let mut draw_order: Vec<PlayerId> = self.attackers.clone();
        draw_order.push(self.defender);

        for pid in draw_order {
            while self.hand_size(pid) < 6 && !self.stock.is_empty() {
                if let Some(card) = self.stock.pop() {
                    // Drawn cards remain private (or become private if they were in stock)
                    // Exception: trump card (first in stock) is already public
                    // Exception: Unknown cards stay unknown (manual mode)
                    let card = if card.is_public() || card.is_unknown() {
                        card
                    } else {
                        card.as_private()
                    };
                    self.hands[pid as usize].push(card);
                }
            }
        }
    }
}

pub fn full_deck(size: usize) -> Vec<Card> {
    let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
    let ranks = match size {
        32 => vec![Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace],
        36 => vec![Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace],
        40 => vec![Rank::Five, Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace],
        44 => vec![Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace],
        48 => vec![Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace],
        52 => vec![Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace],
        _ => vec![Rank::Six, Rank::Seven, Rank::Eight, Rank::Nine, Rank::Ten, Rank::Jack, Rank::Queen, Rank::King, Rank::Ace], // default to 36
    };

    let mut out = Vec::with_capacity(size);
    for s in suits {
        for r in &ranks {
            out.push(Card::private(s, *r));
        }
    }
    out
}

/// Result of rollout-based evaluation for a single action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutActionResult {
    /// The action evaluated.
    pub action: Action,
    /// Number of visits (rollouts) for this action.
    pub visits: u32,
    /// Score: wins / visits. Range [0, 1].
    pub score: f32,
}

// ============================================================================
// UCT-MCTS Tree Search (non-recursive, compact)
// ============================================================================

/// MCTS tree node: wins/visits stats + children keyed by action.
#[derive(Debug, Clone, Default)]
struct MCTSNode {
    wins: u32,
    visits: u32,
    children: HashMap<Action, MCTSNode>,
    unexplored: Vec<Action>,
}

impl MCTSNode {
    /// UCT selection: w/n + C * sqrt(ln(N)/n)
    fn uct_select(&self, c: f64) -> Option<&Action> {
        let ln_n = (self.visits as f64).ln();
        self.children.iter()
            .max_by(|(_, a), (_, b)| {
                let score = |node: &MCTSNode| {
                    if node.visits == 0 { f64::INFINITY }
                    else { node.wins as f64 / node.visits as f64 + c * (ln_n / node.visits as f64).sqrt() }
                };
                score(a).partial_cmp(&score(b)).unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(action, _)| action)
    }
}

/// Result of UCT-MCTS evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCTSEvalAll {
    pub actions: Vec<RolloutActionResult>,
    pub total_rollouts: u32,
}

/// UCT-MCTS tree search. Builds tree adaptively, focusing on promising moves.
/// The state should be determinized before calling this function.
pub fn mcts_evaluate_actions(
    state: &GameState,
    seed: u64,
    perspective: PlayerId,
    rollouts: u32,
    max_depth: u32,
    c: f64,
) -> MCTSEvalAll {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut root = MCTSNode { unexplored: state.legal_actions(), ..Default::default() };

    for _ in 0..rollouts {
        let mut path = vec![];
        let mut node = &mut root;
        let mut s = state.clone();

        // Selection: descend tree using UCT until we find unexplored action or terminal
        while node.unexplored.is_empty() && !node.children.is_empty() && !s.is_terminal() {
            let action = node.uct_select(c).unwrap().clone();
            let _ = s.apply(&action);
            node = node.children.get_mut(&action).unwrap();
            path.push(action);
        }

        // Expansion: if unexplored actions exist, expand one
        if let Some(action) = node.unexplored.pop() {
            let _ = s.apply(&action);
            path.push(action.clone());
            node.children.insert(action, MCTSNode { unexplored: s.legal_actions(), ..Default::default() });
        }

        // Simulation: random playout to terminal
        let mut depth = 0u32;
        while !s.is_terminal() && depth < max_depth {
            let acts = s.legal_actions();
            if acts.is_empty() { break; }
            let idx = rng.gen_range(0..acts.len());
            let _ = s.apply(&acts[idx]);
            depth += 1;
        }
        let win = matches!(s.durak(), Some(d) if d != perspective);

        // Backpropagation: update stats along path
        root.visits += 1;
        if win { root.wins += 1; }
        let mut node = &mut root;
        for action in &path {
            node = node.children.get_mut(action).unwrap();
            node.visits += 1;
            if win { node.wins += 1; }
        }
    }

    // Extract and sort results
    let mut results: Vec<_> = root.children.iter().map(|(action, n)| {
        let score = if n.visits > 0 {
            n.wins as f32 / n.visits as f32
        } else {
            0.0
        };
        RolloutActionResult {
            action: action.clone(),
            visits: n.visits,
            score,
        }
    }).collect();
    results.sort_by(|a, b| b.visits.cmp(&a.visits)
        .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)));

    MCTSEvalAll { actions: results, total_rollouts: rollouts }
}

/// Pick a random legal action from the given state.
/// Returns None if no legal actions are available (terminal state).
pub fn pick_random_action(state: &GameState, seed: u64) -> Option<Action> {
    let actions = state.legal_actions();
    if actions.is_empty() {
        return None;
    }
    let mut rng = StdRng::seed_from_u64(seed);
    Some(actions[rng.gen_range(0..actions.len())].clone())
}
