use durak_core::{mcts_evaluate_actions, pick_random_action as core_pick_random_action, Action, Card, GameState, PlayerId, Rank};

/// Default maximum search depth for MCTS simulations.
const DEFAULT_MAX_DEPTH: u32 = 100;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn init_panic_hook() {
    // In real apps you'd use `console_error_panic_hook`.
    // Keeping deps minimal in backbone.
}

/// Request for creating a new computer game.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewComputerGameRequest {
    /// Seed for random card dealing.
    pub seed: u64,
}

/// Create a new computer game with cards dealt by Rust.
#[wasm_bindgen]
pub fn new_computer_game(req_json: JsValue, config_json: JsValue) -> Result<JsValue, JsValue> {
    let req: NewComputerGameRequest = serde_wasm_bindgen::from_value(req_json)?;
    let config: durak_core::GameConfig = serde_wasm_bindgen::from_value(config_json)
        .unwrap_or_default();
    let state = GameState::new_computer_game(req.seed, config);
    Ok(serde_wasm_bindgen::to_value(&state).unwrap())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManualStartRequest {
    pub trump_card: Card,
    pub player_hand: Vec<Card>,
    pub starting_player: u8,
    /// Opponent declared trumps: list of (PlayerId, Option<Rank>).
    /// None rank means opponent has no trump / didn't declare.
    #[serde(default)]
    pub opponent_trumps: Vec<(PlayerId, Option<Rank>)>,
}

#[wasm_bindgen]
pub fn new_manual_game(req_json: JsValue, config_json: JsValue) -> Result<JsValue, JsValue> {
    let req: ManualStartRequest = serde_wasm_bindgen::from_value(req_json)?;
    let config: durak_core::GameConfig = serde_wasm_bindgen::from_value(config_json)
        .unwrap_or_default();
    let s = GameState::new_manual_game(
        req.trump_card,
        req.player_hand,
        req.starting_player,
        req.opponent_trumps,
        config,
    )
    .map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(serde_wasm_bindgen::to_value(&s).unwrap())
}

/// Get legal actions for a game state.
#[wasm_bindgen]
pub fn legal_actions(state_json: JsValue) -> Result<JsValue, JsValue> {
    let state: GameState = serde_wasm_bindgen::from_value(state_json)?;
    let acts = state.legal_actions();
    Ok(serde_wasm_bindgen::to_value(&acts).unwrap())
}

/// Apply an action to a game state.
#[wasm_bindgen]
pub fn apply_action(state_json: JsValue, action_json: JsValue) -> Result<JsValue, JsValue> {
    let mut state: GameState = serde_wasm_bindgen::from_value(state_json)?;
    let action: Action = serde_wasm_bindgen::from_value(action_json)?;
    state.apply(&action).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(serde_wasm_bindgen::to_value(&state).unwrap())
}

/// Get the actor to move for a game state.
#[wasm_bindgen]
pub fn actor_to_move(state_json: JsValue) -> Result<JsValue, JsValue> {
    let state: GameState = serde_wasm_bindgen::from_value(state_json)?;
    let actor = state.actor_to_move();
    Ok(serde_wasm_bindgen::to_value(&actor).unwrap())
}

/// Check if all cards are deducible from the human player's (P0) perspective.
/// All cards are deducible if:
/// 1. Stock is empty
/// 2. At most one opponent has non-public cards (can be deduced by elimination)
#[wasm_bindgen]
pub fn all_cards_deducible(state_json: JsValue) -> Result<JsValue, JsValue> {
    let state: GameState = serde_wasm_bindgen::from_value(state_json)?;

    // Stock must be empty
    if !state.stock.is_empty() {
        return Ok(serde_wasm_bindgen::to_value(&false).unwrap());
    }

    // Count opponents (non-P0) who have at least one non-public card
    let mut opponents_with_hidden = 0;
    for (i, hand) in state.hands.iter().enumerate() {
        if i == 0 {
            continue; // Skip human player (P0)
        }
        let has_non_public = hand.iter().any(|c| !c.is_public());
        if has_non_public {
            opponents_with_hidden += 1;
        }
    }

    // Deducible if at most 1 opponent has hidden cards
    let deducible = opponents_with_hidden <= 1;
    Ok(serde_wasm_bindgen::to_value(&deducible).unwrap())
}

/// Deduce all unknown cards and return state with them revealed.
/// Only works when all_cards_deducible would return true.
/// Unknown cards are replaced with Public cards deduced by elimination.
#[wasm_bindgen]
pub fn deduce_cards(state_json: JsValue) -> Result<JsValue, JsValue> {
    use std::collections::HashSet;

    let mut state: GameState = serde_wasm_bindgen::from_value(state_json)?;

    // Collect all known cards (suit, rank)
    let mut known: HashSet<(durak_core::Suit, Rank)> = HashSet::new();

    // Cards in hands (public or private)
    for hand in &state.hands {
        for card in hand {
            if card.is_public() || card.is_private() {
                known.insert((card.suit(), card.rank()));
            }
        }
    }

    // Cards on table
    for pile in &state.table {
        known.insert((pile.attack.suit(), pile.attack.rank()));
        if let Some(d) = pile.defense {
            known.insert((d.suit(), d.rank()));
        }
    }

    // Cards in discard
    for card in &state.discard {
        known.insert((card.suit(), card.rank()));
    }

    // Cards in stock (public ones like trump)
    for card in &state.stock {
        if card.is_public() {
            known.insert((card.suit(), card.rank()));
        }
    }

    // Calculate remaining cards (unknown)
    let mut remaining: Vec<Card> = durak_core::full_deck(state.config.deck_size)
        .into_iter()
        .filter(|c| !known.contains(&(c.suit(), c.rank())))
        .map(|c| c.as_public())
        .collect();

    // Replace Unknown cards in hands with remaining cards
    for hand in &mut state.hands {
        for card in hand.iter_mut() {
            if card.is_unknown() {
                if let Some(deduced) = remaining.pop() {
                    *card = deduced;
                }
            }
        }
    }

    // Replace Unknown cards in stock (if any)
    for card in &mut state.stock {
        if card.is_unknown() {
            if let Some(deduced) = remaining.pop() {
                *card = deduced;
            }
        }
    }

    Ok(serde_wasm_bindgen::to_value(&state).unwrap())
}

/// Get the loser (durak) of the game, or null if game is not over.
#[wasm_bindgen]
pub fn get_durak(state_json: JsValue) -> Result<JsValue, JsValue> {
    let state: GameState = serde_wasm_bindgen::from_value(state_json)?;
    Ok(serde_wasm_bindgen::to_value(&state.durak()).unwrap())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAggregate {
    pub action: Action,
    /// Total number of visits (rollouts) across all determinizations.
    pub visits: u32,
    /// Score: wins / visits. Range [0, 1].
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveAggregate {
    pub determinizations: u32,
    /// Total visits across all determinizations.
    pub total_visits: u32,
    /// Score of the best action.
    pub best_score: f32,
    pub actions: Vec<ActionAggregate>,
}

/// Unified solve request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedSolveRequest {
    /// Game state.
    pub state: GameState,
    /// Number of determinizations to run.
    #[serde(default = "default_determinizations")]
    pub determinizations: u32,
    /// Number of MCTS rollouts per determinization.
    #[serde(default)]
    pub rollouts_per_determinization: Option<u32>,
    /// Maximum search depth for rollouts.
    #[serde(default)]
    pub max_depth: Option<u32>,
}

fn default_determinizations() -> u32 { 1 }

/// Create a normalized action key that ignores card visibility (Public/Private/Unknown).
/// This allows matching actions across determinizations where card types may differ.
fn normalize_action_key(action: &Action) -> String {
    match action {
        Action::Attack { card } => format!("attack:{}:{}", card.suit() as u8, card.rank() as u8),
        Action::PassAttack => "pass_attack".to_string(),
        Action::Defend { pile_index, card } => format!("defend:{}:{}:{}", pile_index, card.suit() as u8, card.rank() as u8),
        Action::Take => "take".to_string(),
        Action::Throw { card: Some(c) } => format!("throw:{}:{}", c.suit() as u8, c.rank() as u8),
        Action::Throw { card: None } => "throw:done".to_string(),
        Action::Reflect { card } => format!("reflect:{}:{}", card.suit() as u8, card.rank() as u8),
        Action::ReflectTrump { card } => format!("reflect_trump:{}:{}", card.suit() as u8, card.rank() as u8),
    }
}

/// Unified solve function using UCT-MCTS tree search.
#[wasm_bindgen]
pub fn solve(req_json: JsValue) -> Result<JsValue, JsValue> {
    let req: UnifiedSolveRequest = serde_wasm_bindgen::from_value(req_json)?;

    // Get perspective from actor to move
    let perspective = req.state.actor_to_move();

    // Get legal actions
    let root_actions = req.state.legal_actions();
    if root_actions.is_empty() {
        let out = SolveAggregate {
            determinizations: req.determinizations,
            total_visits: 0,
            best_score: 0.0,
            actions: vec![],
        };
        return Ok(serde_wasm_bindgen::to_value(&out).unwrap());
    }

    // Track results per action across all determinizations
    use std::collections::HashMap;

    #[derive(Default)]
    struct ActionStats {
        visits: u32,
        weighted_score: f32, // sum of (visits * score) for weighted average
    }

    let mut action_stats: HashMap<String, ActionStats> = HashMap::new();
    let mut total_visits = 0u32;

    // Initialize stats for all root actions using normalized keys
    for action in &root_actions {
        let key = normalize_action_key(action);
        action_stats.insert(key, ActionStats::default());
    }

    let max_depth = req.max_depth.unwrap_or(DEFAULT_MAX_DEPTH);
    let rollouts = req.rollouts_per_determinization.unwrap_or(1000);

    // Run MCTS for each determinization
    for i in 0..req.determinizations {
        use rand::{rngs::StdRng, SeedableRng};
        let seed = (js_sys::Math::random() * 1_000_000_000.0) as u64 + i as u64;
        let mut rng = StdRng::seed_from_u64(seed);

        // Determinize the state (assign random cards to unknown slots and stock)
        let det_state = req.state.determinize(perspective, &mut rng);

        // Run MCTS on the determinized state
        let eval = mcts_evaluate_actions(&det_state, seed, perspective, rollouts, max_depth, 1.41);

        total_visits += eval.total_rollouts;

        // Record results for each action using normalized keys
        for result in eval.actions {
            let key = normalize_action_key(&result.action);
            if let Some(stats) = action_stats.get_mut(&key) {
                stats.visits += result.visits;
                stats.weighted_score += result.visits as f32 * result.score;
            }
        }
    }

    // Build the result using normalized keys
    let mut actions: Vec<ActionAggregate> = root_actions
        .into_iter()
        .filter_map(|action| {
            let key = normalize_action_key(&action);
            action_stats.get(&key).map(|stats| {
                let score = if stats.visits > 0 {
                    stats.weighted_score / stats.visits as f32
                } else {
                    0.0
                };
                ActionAggregate {
                    action,
                    visits: stats.visits,
                    score,
                }
            })
        })
        .collect();

    // Sort by score descending (best actions first)
    actions.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    let best_score = actions.first().map(|a| a.score).unwrap_or(0.0);

    let out = SolveAggregate {
        determinizations: req.determinizations,
        total_visits,
        best_score,
        actions,
    };

    Ok(serde_wasm_bindgen::to_value(&out).unwrap())
}

/// Pick a random legal action from the given state.
/// Returns null if no legal actions are available (terminal state).
#[wasm_bindgen]
pub fn pick_random_action(state_json: JsValue) -> Result<JsValue, JsValue> {
    let state: GameState = serde_wasm_bindgen::from_value(state_json)?;
    let seed = (js_sys::Math::random() * 1_000_000_000.0) as u64;
    let action = core_pick_random_action(&state, seed);
    Ok(serde_wasm_bindgen::to_value(&action).unwrap())
}
