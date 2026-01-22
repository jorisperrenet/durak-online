const SETTINGS_KEY = 'durak_game_settings'
const SETTINGS_VERSION = 2

/**
 * Player types for AI behavior configuration.
 */
export const PlayerType = {
  Human: 'Human',
  Random: 'Random',
  MCTS: 'MCTS',
  Real: 'Real', // Real-life player (manual mode)
}

export const DEFAULT_SETTINGS = {
  _version: SETTINGS_VERSION,
  deckSize: 36,
  numPlayers: 2,
  trumpReflecting: true,
  reflecting: true,
  numThreads: Math.max(4, Math.min(8, Math.floor(((typeof navigator !== 'undefined' && navigator.hardwareConcurrency) || 8) / 2))),
  playerNames: {
    P0: 'You',
    P1: 'Player 1',
    P2: 'Player 2',
    P3: 'Player 3',
    P4: 'Player 4',
    P5: 'Player 5',
  },
  // Player types (indexed by player ID)
  playerTypes: {
    P0: PlayerType.Human,
    P1: PlayerType.MCTS,
    P2: PlayerType.MCTS,
    P3: PlayerType.MCTS,
    P4: PlayerType.MCTS,
    P5: PlayerType.MCTS,
  },
  // Computer shuffle toggle: true = computer deals, false = manual input
  computerShuffle: false,
  // MCTS thinking time in milliseconds (for MCTS players)
  mctsThinkingTimeMs: 2000,
  // Number of MCTS determinizations per solve
  mctsDeterminizations: 20,
  // Show hints for human player (continuous MCTS)
  showHints: true,
  // Hint solver settings (determinizations and rollouts per action)
  hintDeterminizations: 50,
  hintRollouts: 5000,
}

/**
 * Validate settings and return any errors.
 * @param {typeof DEFAULT_SETTINGS} settings
 * @returns {{ valid: boolean, errors: string[] }}
 */
export function validateSettings(settings) {
  const errors = []

  // Find human players (only among active players)
  const humanPlayers = Object.entries(settings.playerTypes)
    .filter(([pid]) => parseInt(pid.slice(1)) < settings.numPlayers)
    .filter(([, type]) => type === PlayerType.Human)
    .map(([pid]) => pid)

  // Allow zero human players (computer plays itself)
  // But if there is a human, it must be P0
  if (humanPlayers.length > 1) {
    errors.push('Only one human player is allowed (to prevent card peeking)')
  } else if (humanPlayers.length === 1 && humanPlayers[0] !== 'P0') {
    errors.push('Human player must be P0 (use your own device to play as a different player)')
  }

  return { valid: errors.length === 0, errors }
}

/**
 * Get the human player ID, or null if none.
 * @param {typeof DEFAULT_SETTINGS} settings
 * @returns {string | null}
 */
export function getHumanPlayer(settings) {
  for (let i = 0; i < settings.numPlayers; i++) {
    const pid = `P${i}`
    if (settings.playerTypes[pid] === PlayerType.Human) {
      return pid
    }
  }
  return null
}

/**
 * Load settings from localStorage
 * @returns {typeof DEFAULT_SETTINGS}
 */
export function loadSettings() {
  try {
    const stored = localStorage.getItem(SETTINGS_KEY)
    if (stored) {
      const parsed = JSON.parse(stored)

      // Check version and migrate if needed
      if (!parsed._version || parsed._version < SETTINGS_VERSION) {
        return migrateSettings(parsed)
      }

      // Deep merge nested objects
      const settings = {
        ...DEFAULT_SETTINGS,
        ...parsed,
        playerNames: {
          ...DEFAULT_SETTINGS.playerNames,
          ...(parsed.playerNames || {}),
        },
        playerTypes: {
          ...DEFAULT_SETTINGS.playerTypes,
          ...(parsed.playerTypes || {}),
        },
      }

      // Fix any human players in non-P0 slots (from old versions)
      for (let i = 1; i < 6; i++) {
        const pid = `P${i}`
        if (settings.playerTypes[pid] === PlayerType.Human) {
          settings.playerTypes[pid] = PlayerType.Random
        }
      }

      return settings
    }
  } catch (e) {
    console.warn('Failed to load settings:', e)
  }
  return { ...DEFAULT_SETTINGS }
}

/**
 * Migrate old settings to new format.
 * @param {object} old
 * @returns {typeof DEFAULT_SETTINGS}
 */
function migrateSettings(old) {
  const migrated = { ...DEFAULT_SETTINGS }

  // Preserve existing settings that still exist
  if (old.deckSize) migrated.deckSize = old.deckSize
  if (old.numPlayers) migrated.numPlayers = old.numPlayers
  if (old.trumpReflecting !== undefined) migrated.trumpReflecting = old.trumpReflecting
  if (old.reflecting !== undefined) migrated.reflecting = old.reflecting
  if (old.numThreads) migrated.numThreads = old.numThreads
  if (old.playerNames) {
    migrated.playerNames = { ...DEFAULT_SETTINGS.playerNames, ...old.playerNames }
  }

  console.log('Migrated settings from version', old._version || 1, 'to', SETTINGS_VERSION)
  return migrated
}

/**
 * Save settings to localStorage
 * @param {typeof DEFAULT_SETTINGS} settings
 */
export function saveSettings(settings) {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify({ ...settings, _version: SETTINGS_VERSION }))
  } catch (e) {
    console.warn('Failed to save settings:', e)
  }
}

/**
 * Reset settings to defaults
 * @returns {typeof DEFAULT_SETTINGS}
 */
export function resetSettings() {
  try {
    localStorage.removeItem(SETTINGS_KEY)
  } catch (e) {
    console.warn('Failed to reset settings:', e)
  }
  return { ...DEFAULT_SETTINGS }
}

/**
 * Convert settings to GameConfig format for WASM
 * @param {typeof DEFAULT_SETTINGS} settings
 */
export function settingsToGameConfig(settings) {
  return {
    deck_size: settings.deckSize,
    num_players: settings.numPlayers,
    trump_reflecting: settings.trumpReflecting,
    reflecting: settings.reflecting,
  }
}
