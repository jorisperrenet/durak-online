<script>
  import { onMount, onDestroy } from 'svelte'

  let year = new Date().getFullYear()
  import Card from './lib/Card.svelte'
  import { cardLabel, suitSymbol } from './lib/cards.js'
  import { loadSettings, saveSettings, settingsToGameConfig, PlayerType, getHumanPlayer } from './lib/settings.js'

  import initWasm, {
    new_manual_game,
    new_computer_game,
    pick_random_action,
    legal_actions,
    apply_action,
    all_cards_deducible,
    deduce_cards,
    get_durak,
  } from './wasm/durak_wasm.js'

  // ═══════════════════════════════════════════════════════════════════════════
  // CONSTANTS
  // ═══════════════════════════════════════════════════════════════════════════

  const SUITS = ['Clubs', 'Diamonds', 'Hearts', 'Spades']
  const ALL_RANKS = ['Two', 'Three', 'Four', 'Five', 'Six', 'Seven', 'Eight', 'Nine', 'Ten', 'Jack', 'Queen', 'King', 'Ace']
  const RANK_ORDER = ALL_RANKS
  const RED_SUITS = ['Hearts', 'Diamonds']
  const BLACK_SUITS = ['Clubs', 'Spades']

  // Build alternating suit order with trump first, then alternating colors
  function getAlternatingSuitOrder(trumpSuit) {
    if (!trumpSuit) return ['Hearts', 'Clubs', 'Diamonds', 'Spades']
    const isRed = RED_SUITS.includes(trumpSuit)
    const sameColor = isRed ? RED_SUITS : BLACK_SUITS
    const otherColor = isRed ? BLACK_SUITS : RED_SUITS
    const otherSameColor = sameColor.find(s => s !== trumpSuit)
    // Trump first, then alternate: other color, same color, other color
    return [trumpSuit, otherColor[0], otherSameColor, otherColor[1]]
  }
  const MAX_HISTORY = 50
  const STORAGE_KEY_MANUAL = 'durak_manual_game'
  const STORAGE_KEY_COMPUTER = 'durak_computer_game'

  // ═══════════════════════════════════════════════════════════════════════════
  // STATE
  // ═══════════════════════════════════════════════════════════════════════════

  // Core game state
  let settings = loadSettings()
  let state = null
  let legal = []
  let stateHistory = []
  let wasmReady = false
  let error = ''

  // Solver state
  let workers = []
  let solveId = 0
  let workerResults = []
  let aggregate = null
  let busySolve = false
  let finishedCount = 0
  let solveInterval = null
  let lastActionState = null
  const maxDeterminizations = 500
  const maxRollouts = 50000

  // AI state
  let aiThinking = false
  let aiThinkingPlayer = null
  const aiActionDelay = 500

  // Computer shuffle mode state
  let showOpponentCards = false

  // Manual mode setup state
  let manualGameStarted = false
  let manualTrumpSuit = 'Spades'
  let manualTrumpRank = 'Six'
  let manualStarts = 'P0'  // Player ID who starts (lowest trump holder)
  let manualHand = []
  let opponentTrumps = {}
  let trumpCardSelected = false

  // UI state
  let editingPlayer = null
  let editingName = ''
  let opponentPlaySuit = 'Clubs'
  let opponentPlayRank = 'Six'

  // ═══════════════════════════════════════════════════════════════════════════
  // DERIVED STATE
  // ═══════════════════════════════════════════════════════════════════════════

  $: me = getHumanPlayer(settings)
  $: hasMctsPlayer = Object.entries(settings.playerTypes).some(([pid, type]) => parseInt(pid.slice(1)) < settings.numPlayers && type === PlayerType.MCTS)
  $: playerNames = settings.playerNames || {}
  $: RANKS = ALL_RANKS.slice(-(settings.deckSize / 4))

  // Clean up invalid cards when deck size changes
  $: {
    const validRanks = new Set(RANKS)
    if (manualHand.some(c => !validRanks.has(c.rank))) {
      manualHand = manualHand.filter(c => validRanks.has(c.rank))
    }
    if (!validRanks.has(manualTrumpRank)) {
      manualTrumpRank = RANKS[0]
      trumpCardSelected = false
    }
  }
  $: currentTrumpCard = { suit: manualTrumpSuit, rank: manualTrumpRank }
  $: lowestTrumpInHand = getLowestTrumpInHand(manualHand, manualTrumpSuit, currentTrumpCard)
  $: remainingTrumpCards = getRemainingTrumpCards(manualHand, manualTrumpSuit, manualTrumpRank, opponentTrumps)
  $: step3Complete = Array.from({ length: settings.numPlayers }, (_, i) => `P${i}`).filter(p => p !== me).every(p => opponentTrumps[p])

  // ═══════════════════════════════════════════════════════════════════════════
  // UTILITY FUNCTIONS
  // ═══════════════════════════════════════════════════════════════════════════

  const cardKey = c => `${c?.suit || '?'}:${c?.rank || '?'}`
  const actionKey = a => JSON.stringify(a)
  const delay = ms => new Promise(r => setTimeout(r, ms))
  const actor = st => {
    if (!st) return null
    if (st.phase === 'Defending') return st.defender
    // Attacking or Throwing: use current attacker from attackers list
    if (st.attackers?.length > 0) {
      const idx = st.current_attacker_idx ?? 0
      return st.attackers[idx % st.attackers.length]
    }
    return st.attacker
  }
  const displayName = pid => pid === 'P0' ? 'You' : (playerNames[pid] || pid)

  // Check if a player is still in the game
  function isPlayerActive(st, pid) {
    if (!st) return false
    const idx = parseInt(pid.slice(1))
    const handSize = st.hands?.[idx]?.length || 0
    const stockEmpty = st.stock?.length === 0
    return handSize > 0 || !stockEmpty
  }

  // Get a player's role in the current trick
  function getPlayerRole(st, pid) {
    if (!st) return null
    // Players with no cards (and empty stock) have no role
    if (!isPlayerActive(st, pid)) return null
    if (st.defender === pid) return 'Defender'
    if (st.attackers?.length > 0) {
      const idx = st.attackers.indexOf(pid)
      if (idx === 0) return 'Main Attacker'
      if (idx > 0) {
        const num = idx + 1
        const suffix = num === 1 ? 'st' : num === 2 ? 'nd' : num === 3 ? 'rd' : 'th'
        return `${num}${suffix} Attacker`
      }
    } else if (st.attacker === pid) {
      return 'Main Attacker'
    }
    return null
  }

  // Check if it's this player's turn
  function isPlayersTurn(st, pid) {
    return actor(st) === pid
  }

  // Helper to get hand size - hands are now Vec<Card> where Card is Public/Private/Unknown enum
  function getHandSize(hand) {
    if (!hand || !Array.isArray(hand)) return 0
    return hand.length
  }

  // Helper to parse a Card enum from Rust serialization
  // Returns { suit, rank } or null for Unknown
  function parseCard(card) {
    if (!card) return null
    // Internally tagged format: { type: 'public' | 'private' | 'unknown', suit?, rank? }
    if (card.type === 'unknown') return null
    if (card.type === 'public' || card.type === 'private') return { suit: card.suit, rank: card.rank }
    // Legacy externally tagged format
    if (card.Public) return card.Public
    if (card.Private) return card.Private
    // Fallback for direct object format
    if (card.suit && card.rank) return card
    return null
  }

  // Check if a card is public (known to everyone)
  function isPublicCard(card) {
    if (!card) return false
    return card.type === 'public' || !!card.Public
  }

  // Get public cards from a hand (cards known to everyone, not private)
  function getKnownCards(hand) {
    if (!hand || !Array.isArray(hand)) return []
    // Only return public cards - private cards are not known to opponents
    return hand.filter(isPublicCard).map(parseCard).filter(c => c !== null)
  }

  // Count unknown/private cards in a hand (cards not known to the viewer)
  function getUnknownCount(hand) {
    if (!hand || !Array.isArray(hand)) return 0
    // Count cards that are not public (unknown or private)
    return hand.filter(c => !isPublicCard(c)).length
  }

  function isGameOver(st) {
    if (!st) return false
    try {
      const durak = get_durak(st)
      // Rust None becomes undefined in JS, so check for both null and undefined
      return durak != null
    } catch (e) {
      console.error('isGameOver error:', e)
      return false
    }
  }

  function getLoser(st) {
    if (!st) return null
    try { return get_durak(st) ?? null } catch { return null }
  }

  function areAllCardsDeducible(st) {
    if (!st) return false
    // In manual mode, if the human player has unknown cards, we can't deduce yet
    // The UI must first ask the player to specify those cards
    if (!settings.computerShuffle && myUnknownCount(st) > 0) return false
    try { return all_cards_deducible(st) } catch { return false }
  }

  function sortCards(cards) {
    const trumpSuit = state?.trump || manualTrumpSuit
    const suitOrder = getAlternatingSuitOrder(trumpSuit)
    return [...cards].sort((a, b) => {
      const suitDiff = suitOrder.indexOf(a.suit) - suitOrder.indexOf(b.suit)
      return suitDiff !== 0 ? suitDiff : RANK_ORDER.indexOf(b.rank) - RANK_ORDER.indexOf(a.rank)
    })
  }

  function actionCard(a) {
    if (!a) return null
    if (a.type === 'throw') return a.card ? parseCard(a.card) : null
    return ['attack', 'defend', 'reflect', 'reflect_trump'].includes(a.type) ? parseCard(a.card) : null
  }

  function actionText(a) {
    const labels = { attack: `Attack ${cardLabel(parseCard(a?.card))}`, defend: `Defend ${cardLabel(parseCard(a?.card))}`, pass_attack: 'Pass / end attack', take: 'Take', reflect: `Reflect ${cardLabel(parseCard(a?.card))}`, reflect_trump: `Show Trump ${cardLabel(parseCard(a?.card))}` }
    if (a?.type === 'throw') return a.card ? `Throw ${cardLabel(parseCard(a.card))}` : 'Pass'
    return labels[a?.type] || JSON.stringify(a)
  }

  // Get my full hand - the player knows all their own cards (public and private)
  function myFullHand(st) {
    if (!st) return []
    // Return all cards that have suit/rank (public or private, not unknown)
    return (st.hands?.[0] || []).map(parseCard).filter(c => c !== null)
  }

  const myKnownHand = st => sortCards(myFullHand(st))

  // Count unknown cards in P0's hand (happens in manual mode after drawing)
  function myUnknownCount(st) {
    if (!st) return 0
    const hand = st.hands?.[0] || []
    return hand.filter(c => c?.type === 'unknown' || c?.Unknown !== undefined).length
  }

  function getAllDeckCards() {
    return sortCards(SUITS.flatMap(suit => RANKS.map(rank => ({ suit, rank }))))
  }

  function getAvailableCards(exclude = []) {
    const excludeKeys = new Set(exclude.map(cardKey))
    return getAllDeckCards().filter(c => !excludeKeys.has(cardKey(c)))
  }

  // Get all cards that are known/used in the current game state (for manual mode card picker)
  function getUsedCardsInGame(st) {
    if (!st) return []
    const used = []
    // Trump card (bottom of stock)
    if (st.stock?.length > 0) {
      const trump = parseCard(st.stock[0])
      if (trump) used.push(trump)
    }
    // All known cards in all hands
    for (const hand of st.hands || []) {
      for (const c of hand) {
        const parsed = parseCard(c)
        if (parsed) used.push(parsed)
      }
    }
    // Cards on the table
    for (const pile of st.table || []) {
      if (pile.attack) {
        const parsed = parseCard(pile.attack)
        if (parsed) used.push(parsed)
      }
      if (pile.defense) {
        const parsed = parseCard(pile.defense)
        if (parsed) used.push(parsed)
      }
    }
    // Discard pile
    for (const c of st.discard || []) {
      const parsed = parseCard(c)
      if (parsed) used.push(parsed)
    }
    return used
  }

  // Replace an unknown card in P0's hand with an actual card
  function replaceUnknownCard(cardToAdd) {
    if (!state) return
    const hand = state.hands?.[0] || []
    const unknownIdx = hand.findIndex(c => c?.type === 'unknown' || c?.Unknown !== undefined)
    if (unknownIdx === -1) return

    // Create new hand with the unknown card replaced
    const newHand = [...hand]
    newHand[unknownIdx] = { type: 'private', suit: cardToAdd.suit, rank: cardToAdd.rank }

    // Update state
    const newHands = [...state.hands]
    newHands[0] = newHand
    state = { ...state, hands: newHands }

    refreshLegal()
    saveGameToLocalStorage()

    // If all unknown cards are now specified and it's our turn, start solving
    if (myUnknownCount(state) === 0 && actor(state) === me) {
      solve()
    }
  }

  function getLowestTrumpInHand(hand, trumpSuit, trumpCard) {
    const trumps = hand.filter(c => c.suit === trumpSuit && cardKey(c) !== cardKey(trumpCard))
    if (!trumps.length) return null
    return trumps.sort((a, b) => RANK_ORDER.indexOf(a.rank) - RANK_ORDER.indexOf(b.rank))[0]
  }

  function getRemainingTrumpCards(hand, trumpSuit, trumpRank, selections) {
    const used = new Set([
      ...hand.filter(c => c.suit === trumpSuit).map(cardKey),
      cardKey({ suit: trumpSuit, rank: trumpRank }),
      ...Object.values(selections).filter(r => r && r !== 'None').map(r => cardKey({ suit: trumpSuit, rank: r }))
    ])
    return getAllDeckCards().filter(c => c.suit === trumpSuit && !used.has(cardKey(c))).sort((a, b) => RANK_ORDER.indexOf(a.rank) - RANK_ORDER.indexOf(b.rank))
  }

  function splitWork(total, parts) {
    const per = Math.floor(total / parts), rem = total - per * parts
    return Array.from({ length: parts }, (_, i) => per + (i < rem ? 1 : 0))
  }

  // Svelte action to focus and select all text
  function selectAll(node) { node.focus(); node.select() }

  // ═══════════════════════════════════════════════════════════════════════════
  // LOCAL STORAGE
  // ═══════════════════════════════════════════════════════════════════════════

  function saveGameToLocalStorage() {
    try {
      const key = settings.computerShuffle ? STORAGE_KEY_COMPUTER : STORAGE_KEY_MANUAL
      if (settings.computerShuffle) {
        if (!state) return
        localStorage.setItem(key, JSON.stringify({ mode: 'computer', state, stateHistory, me }))
      } else {
        localStorage.setItem(key, JSON.stringify({
          mode: 'manual', state, stateHistory, opponentTrumps, manualTrumpSuit, manualTrumpRank,
          manualStarts, manualHand, manualGameStarted, trumpCardSelected, me
        }))
      }
    } catch (e) { console.warn('Failed to save game:', e) }
  }

  function loadGameFromLocalStorage(forComputerShuffle) {
    const key = forComputerShuffle ? STORAGE_KEY_COMPUTER : STORAGE_KEY_MANUAL
    try {
      const saved = localStorage.getItem(key)
      if (!saved) return false
      const data = JSON.parse(saved)

      if (forComputerShuffle) {
        if (!data.state?.hands || !Array.isArray(data.state.hands)) {
          localStorage.removeItem(key)
          return false
        }
        try { legal_actions(data.state) } catch { localStorage.removeItem(key); return false }
        state = data.state
        stateHistory = data.stateHistory || []
        refreshLegal()
        handleTurn()
      } else {
        opponentTrumps = data.opponentTrumps || {}
        manualTrumpSuit = data.manualTrumpSuit || 'Spades'
        manualTrumpRank = data.manualTrumpRank || 'Six'
        // Handle backwards compat: old saves had boolean, new saves have player ID
        manualStarts = typeof data.manualStarts === 'string' ? data.manualStarts : (data.manualStarts ? 'P0' : 'P1')
        manualHand = data.manualHand || []
        trumpCardSelected = data.trumpCardSelected || false

        if (data.state?.hands && Array.isArray(data.state.hands)) {
          // Check if there are Unknown cards in P0's hand (skip validation if so)
          const hasUnknown = (data.state.hands[0] || []).some(c => c?.type === 'unknown' || c?.Unknown !== undefined)
          if (!hasUnknown) {
            try { legal_actions(data.state) } catch { manualGameStarted = false; state = null; return true }
          }
          state = data.state
          // Deduce unknown cards if possible
          if (areAllCardsDeducible(state)) {
            try { state = deduce_cards(state) } catch {}
          }
          stateHistory = data.stateHistory || []
          manualGameStarted = data.manualGameStarted ?? true
          refreshLegal()
          // Trigger solve for hints if it's human's turn in manual mode (and no unknown cards)
          if (myUnknownCount(state) === 0 && actor(state) === me) solve()
        } else {
          manualGameStarted = false
          state = null
        }
      }
      return true
    } catch (e) { console.warn('Failed to load game:', e) }
    return false
  }

  function clearSavedGame() {
    localStorage.removeItem(settings.computerShuffle ? STORAGE_KEY_COMPUTER : STORAGE_KEY_MANUAL)
    stateHistory = []
    error = ''
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // WORKERS & SOLVER
  // ═══════════════════════════════════════════════════════════════════════════

  function terminateAllWorkers() {
    workers.forEach(w => { try { w.terminate() } catch {} })
    workers = []
  }

  function getExpectedWorkerCount() {
    return Math.max(1, settings.numThreads || 4)
  }

  function spawnWorkers() {
    terminateAllWorkers()
    finishedCount = 0
    const count = getExpectedWorkerCount()

    for (let i = 0; i < count; i++) {
      try {
        const w = new Worker(new URL('./workers/solver.worker.js', import.meta.url), { type: 'module' })
        w.onmessage = e => {
          const msg = e.data
          if (msg?.type === 'error') {
            if (msg.id && msg.id !== solveId) return
            error = msg.message?.includes('memory') ? 'Out of memory. Try refreshing or reducing threads.' : `[${msg.context}] ${msg.message}`
            markFinished()
          } else if (msg?.type === 'result') {
            if (msg.id !== solveId) return
            workerResults = [...workerResults, { idx: i, elapsedMs: msg.elapsedMs, result: msg.result }]
            recomputeAggregate()
            markFinished()
          }
        }
        w.onerror = e => {
          error = String(e?.message || e).includes('memory') ? 'Out of memory. Try refreshing or reducing threads.' : String(e?.message || e)
          markFinished()
        }
        workers.push(w)
      } catch (e) {
        error = `Failed to create worker: ${e?.message || e}`
        break
      }
    }
  }

  function markFinished() {
    if (++finishedCount >= workers.length) busySolve = false
  }

  function recomputeAggregate() {
    if (!workerResults.length) { aggregate = null; return }

    const m = new Map()
    // WASM solver returns: { action, visits, score } where score = wins/visits (0-1)
    legal.forEach(a => m.set(actionKey(a), { action: a, visits: 0, weightedScore: 0 }))

    let det = 0, totalVisits = 0
    for (const wr of workerResults) {
      const r = wr.result
      det += r.determinizations || 0
      totalVisits += r.total_visits || 0
      for (const a of r.actions || []) {
        const cur = m.get(actionKey(a.action)) || { action: a.action, visits: 0, weightedScore: 0 }
        cur.visits += a.visits || 0
        cur.weightedScore += (a.visits || 0) * (a.score || 0)
        m.set(actionKey(a.action), cur)
      }
    }

    const actions = [...m.values()].map(x => {
      const score = x.visits > 0 ? x.weightedScore / x.visits : null
      // Convert score (0-1 win rate) to mean_score (-1 to 1) for display compatibility
      const mean_score = score !== null ? (score * 2 - 1) : null
      return { ...x, score, mean_score, wins: x.visits, losses: 0, draws: 0 }
    }).sort((a, b) => {
      if (a.mean_score === null && b.mean_score === null) return 0
      if (a.mean_score === null) return 1
      if (b.mean_score === null) return -1
      return b.mean_score - a.mean_score
    })

    aggregate = { determinizations: det, total_visits: totalVisits, mean_score: actions.find(a => a.mean_score !== null)?.mean_score ?? 0, actions }
  }

  function resetSolveUi() {
    solveId++  // Ignore any pending results from workers
    workerResults = []
    aggregate = null
    busySolve = false
    stopSolving()
  }

  function stopSolving() {
    if (solveInterval) { clearInterval(solveInterval); solveInterval = null }
  }

  function toggleHints() {
    settings = { ...settings, showHints: !settings.showHints }
    saveSettings(settings)
    if (settings.showHints && state && actor(state) === me && !isGameOver(state)) {
      solve()
    } else {
      stopSolving()
      solveId++  // Ignore any pending results from workers
      busySolve = false
      workerResults = []
      aggregate = null
    }
  }

  function solve() {
    if (!settings.showHints) return
    if (!wasmReady || !state) return
    if (settings.computerShuffle && actor(state) !== me) return
    if (myUnknownCount(state) > 0) return  // Don't solve while player has unknown cards

    if (workers.length !== getExpectedWorkerCount()) spawnWorkers()
    if (!workers.length) { error = 'No workers available'; return }

    error = ''

    busySolve = true
    finishedCount = 0
    solveId++

    const chunks = splitWork(settings.hintDeterminizations, workers.length)
    for (let i = 0; i < workers.length; i++) {
      if (chunks[i] === 0) continue
      workers[i].postMessage({ type: 'solve', id: solveId, req: { state, determinizations: chunks[i], rollouts_per_determinization: settings.hintRollouts, max_depth: 500 } })
    }

    if (!solveInterval) {
      lastActionState = JSON.stringify({ state, me })
      solveInterval = setInterval(() => {
        if (JSON.stringify({ state, me }) !== lastActionState) { stopSolving(); return }
        if (!busySolve && state && actor(state) === me && myUnknownCount(state) === 0) solve()
      }, 1000)
    }
  }

  // MCTS for AI
  async function runMctsForAI() {
    if (!wasmReady || !state || !workers.length) return null

    const timeLimit = settings.mctsThinkingTimeMs || 2000
    const startTime = performance.now()
    let currentDet = settings.mctsDeterminizations || 20
    let currentRollouts = 5000
    const allResults = new Map()

    while (performance.now() - startTime < timeLimit) {
      const iterStart = performance.now()
      const results = await runMctsIteration(currentDet, currentRollouts)

      for (const r of results) {
        const key = actionKey(r.action)
        const ex = allResults.get(key) || { action: r.action, wins: 0, losses: 0, draws: 0 }
        ex.wins += r.wins || 0
        ex.losses += r.losses || 0
        ex.draws += r.draws || 0
        allResults.set(key, ex)
      }

      const remaining = timeLimit - (performance.now() - startTime)
      if (remaining < 200) break

      const iterTime = performance.now() - iterStart
      if (iterTime < remaining / 2) {
        const scale = Math.min(2, remaining / iterTime / 2)
        currentDet = Math.min(maxDeterminizations, Math.ceil(currentDet * scale))
        currentRollouts = Math.min(maxRollouts, Math.ceil(currentRollouts * Math.sqrt(scale)))
      }
    }

    let best = null, bestScore = -Infinity
    for (const r of allResults.values()) {
      const total = r.wins + r.losses + r.draws
      if (total > 0) {
        const score = (r.wins - r.losses) / total
        if (score > bestScore) { bestScore = score; best = r.action }
      }
    }
    return best
  }

  function runMctsIteration(det, rollouts) {
    return new Promise(resolve => {
      const chunks = splitWork(det, workers.length)
      const results = []
      let finished = 0
      const iterId = ++solveId

      const handler = e => {
        if (e.data.id !== iterId || e.data.type !== 'result') return
        results.push(...(e.data.result?.actions || []))
        if (++finished >= chunks.filter(c => c > 0).length) resolve(results)
      }

      workers.forEach((w, i) => {
        w.addEventListener('message', handler)
        if (chunks[i] > 0) w.postMessage({ type: 'solve', id: iterId, req: { state, determinizations: chunks[i], rollouts_per_determinization: rollouts, max_depth: 500 } })
      })

      setTimeout(() => { workers.forEach(w => w.removeEventListener('message', handler)); resolve(results) }, 10000)
    })
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // GAME LOGIC
  // ═══════════════════════════════════════════════════════════════════════════

  function refreshLegal() {
    // Don't call legal_actions if there are Unknown cards in the human's hand AND it's their turn
    // (calling .suit() or .rank() on Unknown cards causes a panic in Rust)
    if (!wasmReady || !state) {
      legal = []
      return
    }
    // In manual mode, if P0 has Unknown cards and it's P0's turn, can't compute legal actions
    if (!settings.computerShuffle && myUnknownCount(state) > 0 && actor(state) === me) {
      legal = []
      return
    }
    legal = legal_actions(state)
  }

  function switchMode(toComputerShuffle) {
    if (settings.computerShuffle === toComputerShuffle) return

    // Save current game BEFORE changing mode
    saveGameToLocalStorage()

    // Update mode
    settings = { ...settings, computerShuffle: toComputerShuffle }
    saveSettings(settings)

    // Reset UI
    resetSolveUi()
    stateHistory = []
    error = ''

    // Load game for new mode (if none exists, user will click Start Game button)
    if (!loadGameFromLocalStorage(toComputerShuffle)) {
      state = null
      manualGameStarted = false
      legal = []  // Clear legal actions when no game loaded
    }

    // Save to ensure mode is persisted
    saveGameToLocalStorage()
  }

  function newGame() {
    if (!wasmReady) return

    error = ''
    resetSolveUi()
    manualGameStarted = false
    stateHistory = []
    aiThinking = false
    aiThinkingPlayer = null

    try {
      if (!settings.computerShuffle) {
        if (manualHand.length !== 6) { error = 'You must add exactly 6 cards to your hand'; return }
        const config = settingsToGameConfig(settings)
        const oppTrumps = Object.entries(opponentTrumps).filter(([p, r]) => p !== me && r && r !== 'None').map(([p, r]) => [p, r])
        // Wrap cards in Card enum format (internally tagged with type field)
        const wrapCard = c => ({ type: 'public', suit: c.suit, rank: c.rank })
        const startingPlayerIdx = parseInt(manualStarts.replace('P', ''), 10) || 0
        state = new_manual_game({ trump_card: wrapCard(currentTrumpCard), player_hand: manualHand.map(wrapCard), starting_player: startingPlayerIdx, opponent_trumps: oppTrumps }, config)
        manualGameStarted = true
        // Trigger solve for hints if it's human's turn
        if (actor(state) === me) solve()
      } else {
        // Create the game state with a random seed - Rust handles shuffling
        const config = settingsToGameConfig(settings)
        const seed = BigInt(Math.floor(Math.random() * Number.MAX_SAFE_INTEGER))
        state = new_computer_game({ seed }, config)
        handleTurn()
      }
      refreshLegal()
      saveGameToLocalStorage()
    } catch (e) {
      error = String(e?.message || e)
      state = null
    }
  }

  async function handleTurn() {
    if (!state || isGameOver(state)) return

    const current = actor(state)
    if (!current) return

    if (!settings.computerShuffle) {
      if (current === me) solve()
      return
    }

    const playerType = settings.playerTypes[current] || PlayerType.Random
    if (playerType === PlayerType.Human) {
      // Auto-play if only one legal action
      if (legal.length === 1) {
        await delay(200)  // Brief delay so it doesn't feel instant
        if (!state || isGameOver(state) || actor(state) !== current) return
        applyAnyAction(legal[0], true)  // Save to history for human auto-play
        return
      }
      solve()
      return
    }

    aiThinking = true
    aiThinkingPlayer = current

    try {
      if (playerType === PlayerType.MCTS) {
        // Ensure workers are spawned for MCTS AI
        if (workers.length !== getExpectedWorkerCount()) spawnWorkers()
        const action = await runMctsForAI() || pick_random_action(state)
        if (action) applyAnyAction(action)
      } else {
        // Random or any other type defaults to random play
        await delay(aiActionDelay)
        if (!state || isGameOver(state) || actor(state) !== current) return
        const action = pick_random_action(state)
        if (action) applyAnyAction(action)
      }
    } catch (e) {
      error = `AI error: ${e?.message || e}`
      const fallback = pick_random_action(state)
      if (fallback) applyAnyAction(fallback)
    } finally {
      aiThinking = false
      aiThinkingPlayer = null
    }
  }

  function applyAnyAction(action, saveToHistory = false) {
    if (!wasmReady || !state) return
    error = ''

    try {
      if (saveToHistory) {
        stateHistory = [...stateHistory.slice(-MAX_HISTORY + 1), JSON.parse(JSON.stringify(state))]
      }

      state = apply_action(state, action)

      // In manual mode, deduce unknown cards when possible
      if (!settings.computerShuffle && areAllCardsDeducible(state)) {
        try { state = deduce_cards(state) } catch {}
      }

      refreshLegal()
      saveGameToLocalStorage()
      workerResults = []
      aggregate = null
      stopSolving()  // Clear old interval so handleTurn can start fresh
      setTimeout(handleTurn, 0)  // Defer to next tick to avoid race with previous handleTurn's finally
    } catch (e) {
      error = String(e?.message || e)
      if (stateHistory.length) {
        state = stateHistory.pop()
        stateHistory = [...stateHistory]
      }
    }
  }

  function playAction(action) {
    if (actor(state) !== me) return
    if (myUnknownCount(state) > 0) return  // Can't play with unknown cards
    workerResults = []
    aggregate = null
    applyAnyAction(action, true)  // Save to history for human actions
  }

  function playCard(card) {
    const a = legal.find(x => actionCard(x)?.suit === card.suit && actionCard(x)?.rank === card.rank)
    if (a) playAction(a)
  }

  function undo() {
    if (!stateHistory.length) { error = 'No moves to undo'; return }
    error = ''
    state = stateHistory.pop()
    stateHistory = [...stateHistory]
    refreshLegal()
    resetSolveUi()
    aiThinking = false
    aiThinkingPlayer = null
    saveGameToLocalStorage()
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MANUAL MODE HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  function setOpponentTrump(playerId, rank) { opponentTrumps = { ...opponentTrumps, [playerId]: rank } }
  function clearOpponentTrump(playerId) { opponentTrumps = { ...opponentTrumps, [playerId]: 'None' } }

  function addPlayer() {
    if (settings.numPlayers < 6) { settings = { ...settings, numPlayers: settings.numPlayers + 1 }; saveSettings(settings) }
  }
  function removePlayer() {
    if (settings.numPlayers > 2) { settings = { ...settings, numPlayers: settings.numPlayers - 1 }; saveSettings(settings) }
  }
  function updatePlayerName(playerId, name) {
    settings = { ...settings, playerNames: { ...settings.playerNames, [playerId]: name || playerId } }
    saveSettings(settings)
    editingPlayer = null
  }

  function applyOpponentCard() {
    if (!state) return
    const c = { suit: opponentPlaySuit, rank: opponentPlayRank }
    const a = legal.find(x => actionCard(x)?.suit === c.suit && actionCard(x)?.rank === c.rank)
    if (!a) { error = `${cardLabel(c)} is not legal right now`; return }
    applyAnyAction(a, true)  // Save to history for undo
  }

  // Auto-determine starter based on lowest trump
  $: {
    let lowestPlayer = me
    let lowestIdx = lowestTrumpInHand ? RANK_ORDER.indexOf(lowestTrumpInHand.rank) : Infinity
    for (const [player, rank] of Object.entries(opponentTrumps)) {
      if (rank && rank !== 'None') {
        const idx = RANK_ORDER.indexOf(rank)
        if (idx < lowestIdx) { lowestIdx = idx; lowestPlayer = player }
      }
    }
    manualStarts = lowestPlayer
  }

  // Auto-save manual setup
  $: if (!settings.computerShuffle && !manualGameStarted && wasmReady) {
    void (manualHand, manualTrumpSuit, manualTrumpRank, opponentTrumps, trumpCardSelected)
    saveGameToLocalStorage()
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // LIFECYCLE
  // ═══════════════════════════════════════════════════════════════════════════

  onMount(async () => {
    try {
      await initWasm()
      wasmReady = true

      // Use settings.computerShuffle as source of truth for mode
      // Try to load saved game; if none exists, user will click "Start Game" button
      loadGameFromLocalStorage(settings.computerShuffle)
      // Manual mode with no saved game: user sets up game via wizard
    } catch (e) {
      error = `Failed to init WASM: ${e?.message || e}`
    }

    const cleanup = () => terminateAllWorkers()
    window.addEventListener('beforeunload', cleanup)
    return () => window.removeEventListener('beforeunload', cleanup)
  })

  onDestroy(() => { terminateAllWorkers(); stopSolving() })

  // ═══════════════════════════════════════════════════════════════════════════
  // TEMPLATE HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  function isCardPlayable(card) {
    return actor(state) === me && legal.some(a => actionCard(a)?.suit === card.suit && actionCard(a)?.rank === card.rank)
  }

  function bestActionForCard(card) {
    const best = aggregate?.actions?.[0]
    if (!best) return false
    const c = actionCard(best.action)
    return c && c.suit === card.suit && c.rank === card.rank
  }

  function isCardKnownToOpponents(st, card) {
    // Cards known to opponents are Public cards in P0's hand
    // Handle both internally tagged (type: 'public') and externally tagged formats
    return st?.hands?.[0]?.some(c => {
      if (c?.type === 'public') return cardKey({ suit: c.suit, rank: c.rank }) === cardKey(card)
      if (c?.Public) return cardKey(c.Public) === cardKey(card)
      return false
    }) || false
  }

  // Get actions split by type for display
  function getActionGroups(actions) {
    const cardActions = actions.filter(a => {
      if (a.action.type === 'throw') return a.action.card != null
      return ['attack', 'defend', 'reflect', 'reflect_trump'].includes(a.action.type)
    })
    const otherActions = actions.filter(a => a.action.type === 'pass_attack' || a.action.type === 'take' || (a.action.type === 'throw' && a.action.card == null))
    return { cardActions, otherActions }
  }

  function getActionsByType(actions) {
    return {
      attack: actions.filter(a => a.type === 'attack'),
      defend: actions.filter(a => a.type === 'defend'),
      throw: actions.filter(a => a.type === 'throw' && a.card != null),
      reflect: actions.filter(a => a.type === 'reflect'),
      reflectTrump: actions.filter(a => a.type === 'reflect_trump'),
      other: actions.filter(a => !actionCard(a))
    }
  }

  function sortActionsByCard(actions) {
    const trumpSuit = state?.trump || manualTrumpSuit
    const suitOrder = getAlternatingSuitOrder(trumpSuit)
    return [...actions].sort((a, b) => {
      const ca = actionCard(a), cb = actionCard(b)
      if (!ca || !cb) return 0
      const sd = suitOrder.indexOf(ca.suit) - suitOrder.indexOf(cb.suit)
      return sd !== 0 ? sd : RANK_ORDER.indexOf(cb.rank) - RANK_ORDER.indexOf(ca.rank)
    })
  }
</script>

<div class="min-h-screen flex flex-col">
  <main class="flex-1">
  <div class="mx-auto max-w-7xl px-4 py-4">
    <!-- Header -->
    <div class="flex items-center justify-between gap-4">
      <button class="text-lg font-semibold tracking-tight text-zinc-300 hover:text-zinc-100 transition-colors" on:click={() => { localStorage.clear(); location.reload() }}>Durak</button>

      <div class="flex items-center gap-2">
        <span class="text-[10px] text-zinc-600 italic">{settings.computerShuffle ? 'toggle for real game help' : 'toggle to play vs computer'}</span>
        <label class="flex items-center gap-2 cursor-pointer">
          <span class="text-xs text-zinc-400">Computer Shuffle</span>
          <button
            class="relative w-10 h-5 rounded-full transition-colors {settings.computerShuffle ? 'bg-indigo-600' : 'bg-zinc-700'}"
            on:click={() => switchMode(!settings.computerShuffle)}
            role="switch"
            aria-checked={settings.computerShuffle}
            aria-label="Toggle computer shuffle"
          >
            <span class="absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full transition-transform {settings.computerShuffle ? 'translate-x-5' : 'translate-x-0'}"></span>
          </button>
        </label>
      </div>

      <label class="flex items-center gap-1.5 text-xs text-zinc-400">
        <span>Threads:</span>
        <input
          type="number"
          min="1"
          max="16"
          class="w-8 rounded border border-zinc-700 bg-zinc-950 px-1 py-1 text-xs text-center [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
          value={settings.numThreads}
          on:change={e => { settings = { ...settings, numThreads: parseInt(e.target.value) || 4 }; saveSettings(settings) }}
        />
      </label>
    </div>

    {#if error}
      <div class="mt-3 rounded-lg border border-red-900/60 bg-red-950/40 p-3 text-sm text-red-200">
        <span class="font-medium">Error:</span> <span class="font-mono text-xs">{error}</span>
      </div>
    {/if}

    <!-- Controls -->
    <div class="mt-3 flex flex-col gap-2 text-sm">
      <div class="flex items-center gap-2 flex-wrap">
        <span class="text-zinc-500 text-xs">Players:</span>
        {#each Array(settings.numPlayers) as _, i}
          {@const pid = `P${i}`}
          {@const pType = settings.computerShuffle ? (settings.playerTypes[pid] || (i === 0 ? PlayerType.Human : PlayerType.Random)) : (i === 0 ? PlayerType.Human : PlayerType.Real)}
          {@const isHuman = pType === PlayerType.Human}
          <div class="flex items-center gap-0.5 rounded-md border {isHuman ? 'border-indigo-600 bg-indigo-950/30' : 'border-zinc-700 bg-zinc-950'} overflow-hidden">
            {#if i === 0 && isHuman}
              <span class="px-2 py-1 text-xs font-medium text-indigo-300">You</span>
            {:else if editingPlayer === pid}
              <input type="text" class="w-16 px-1.5 py-1 text-xs bg-zinc-800 border-none text-white focus:outline-none" bind:value={editingName}
                on:blur={() => updatePlayerName(pid, editingName)} on:keydown={e => { if (e.key === 'Enter') updatePlayerName(pid, editingName); if (e.key === 'Escape') editingPlayer = null }} use:selectAll />
            {:else}
              <button class="px-2 py-1 text-xs font-medium text-zinc-300 hover:bg-zinc-800" on:click={() => { editingPlayer = pid; editingName = playerNames[pid] || pid }} title="Click to rename">{playerNames[pid] || pid}</button>
            {/if}
            {#if settings.computerShuffle && i > 0}
              <select class="h-6 px-1 text-xs bg-zinc-900 border-l border-zinc-700 text-zinc-400 focus:outline-none cursor-pointer" value={pType}
                on:change={e => { settings = { ...settings, playerTypes: { ...settings.playerTypes, [pid]: e.target.value } }; saveSettings(settings) }}>
                <option value={PlayerType.Random}>Random</option>
                <option value={PlayerType.MCTS}>MCTS</option>
              </select>
            {:else if settings.computerShuffle && i === 0}
              <span class="h-6 flex items-center px-1.5 text-xs border-l border-zinc-700 text-indigo-400">Human</span>
            {:else}
              <span class="h-6 flex items-center px-1.5 text-xs border-l border-zinc-700 text-indigo-400">{pType}</span>
            {/if}
          </div>
        {/each}

        {#if settings.computerShuffle ? !state : !manualGameStarted}
          <button class="w-6 h-6 flex items-center justify-center rounded text-xs bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200 disabled:opacity-30" on:click={removePlayer} disabled={settings.numPlayers <= 2}>−</button>
          <button class="w-6 h-6 flex items-center justify-center rounded text-xs bg-zinc-800 text-zinc-400 hover:bg-zinc-700 hover:text-zinc-200 disabled:opacity-30" on:click={addPlayer} disabled={settings.numPlayers >= 6}>+</button>
        {/if}

        <div class="flex items-center gap-1.5 ml-auto">
          <button class="rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs hover:bg-zinc-800 disabled:opacity-50" on:click={undo} disabled={!stateHistory.length}>
            Undo{stateHistory.length ? ` (${stateHistory.length})` : ''}
          </button>
          <button class="rounded border border-zinc-600 bg-zinc-800 px-2 py-1 text-xs text-zinc-300 hover:bg-zinc-700" on:click={() => { clearSavedGame(); location.reload() }}>Reset</button>
        </div>
      </div>

      {#if settings.computerShuffle && hasMctsPlayer}
        <div class="flex items-center gap-2">
          <label class="flex items-center gap-1 text-xs text-zinc-400">
            <span>Time:</span>
            <input type="number" min="500" max="30000" step="500" class="w-16 px-1 py-0.5 rounded text-xs bg-zinc-800 border border-zinc-700" value={settings.mctsThinkingTimeMs}
              on:change={e => { settings = { ...settings, mctsThinkingTimeMs: parseInt(e.target.value) || 2000 }; saveSettings(settings) }} />
            <span class="text-zinc-500">ms</span>
          </label>
          <label class="flex items-center gap-1 text-xs text-zinc-400">
            <span>Determinizations:</span>
            <input type="number" min="1" max="100" class="w-12 px-1 py-0.5 rounded text-xs bg-zinc-800 border border-zinc-700" value={settings.mctsDeterminizations}
              on:change={e => { settings = { ...settings, mctsDeterminizations: parseInt(e.target.value) || 20 }; saveSettings(settings) }} />
          </label>
        </div>
      {/if}
    </div>

    <!-- Manual Mode Setup Wizard -->
    {#if !settings.computerShuffle && !manualGameStarted}
      <div class="mt-3 rounded-xl border border-zinc-800 bg-zinc-900/40 p-4 space-y-4">
        <!-- Game Rules -->
        <div class="rounded-lg border border-zinc-800 bg-zinc-950/30 p-4">
          <div class="text-sm font-medium text-zinc-300 mb-3">Game Rules</div>
          <div class="grid gap-4 sm:grid-cols-3">
            <!-- Deck Size -->
            <label class="block">
              <span class="text-xs text-zinc-400">Deck Size</span>
              <select
                class="mt-1 w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5 text-sm"
                value={settings.deckSize}
                on:change={e => { settings = { ...settings, deckSize: parseInt(e.target.value) }; saveSettings(settings) }}
              >
                <option value={32}>32 cards (7-Ace)</option>
                <option value={36}>36 cards (6-Ace)</option>
                <option value={40}>40 cards (5-Ace)</option>
                <option value={44}>44 cards (4-Ace)</option>
                <option value={48}>48 cards (3-Ace)</option>
                <option value={52}>52 cards (2-Ace)</option>
              </select>
            </label>

            <!-- Trump Reflecting -->
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-zinc-700 bg-zinc-950 text-indigo-600"
                checked={settings.trumpReflecting}
                on:change={e => { settings = { ...settings, trumpReflecting: e.target.checked }; saveSettings(settings) }}
              />
              <div>
                <span class="text-sm text-zinc-300">Trump Reflecting</span>
                <p class="text-[10px] text-zinc-500">Show trump to redirect</p>
              </div>
            </label>

            <!-- Reflecting -->
            <label class="flex items-center gap-2 cursor-pointer">
              <input
                type="checkbox"
                class="h-4 w-4 rounded border-zinc-700 bg-zinc-950 text-indigo-600"
                checked={settings.reflecting}
                on:change={e => { settings = { ...settings, reflecting: e.target.checked }; saveSettings(settings) }}
              />
              <div>
                <span class="text-sm text-zinc-300">Reflecting</span>
                <p class="text-[10px] text-zinc-500">Same rank to redirect</p>
              </div>
            </label>
          </div>
        </div>

        <!-- Step 1: Trump Card -->
        <div class="rounded-lg border border-zinc-800 bg-zinc-950/30 p-4">
          <div class="flex items-center gap-2 mb-3">
            <span class="flex items-center justify-center w-6 h-6 rounded-full {trumpCardSelected ? 'bg-green-600' : 'bg-indigo-600'} text-xs font-bold">1</span>
            <span class="text-sm font-medium text-zinc-300">Trump card (bottom of deck)</span>
            {#if trumpCardSelected}
              <button class="ml-auto rounded-md border border-zinc-700 bg-zinc-950 px-3 py-1 text-xs font-medium hover:bg-zinc-900" on:click={() => trumpCardSelected = false}>Change</button>
            {/if}
          </div>
          {#if !trumpCardSelected}
            <div class="text-xs text-zinc-500 mb-3">Select the card at the bottom of the deck.</div>
            <div class="grid grid-cols-6 sm:grid-cols-9 md:grid-cols-12 gap-1 max-h-64 overflow-y-auto p-1">
              {#each getAllDeckCards() as c (cardKey(c))}
                <Card card={c} size="sm" selectable on:click={() => { manualTrumpSuit = c.suit; manualTrumpRank = c.rank; trumpCardSelected = true }} />
              {/each}
            </div>
          {:else}
            <div class="flex items-center gap-3">
              <Card card={currentTrumpCard} size="md" />
              <div>
                <div class="text-sm text-zinc-300 font-medium">{cardLabel(currentTrumpCard)}</div>
                <div class="text-xs text-zinc-500 mt-1">Trump: <span class="text-amber-400 font-medium">{suitSymbol(manualTrumpSuit)} {manualTrumpSuit}</span></div>
              </div>
            </div>
          {/if}
        </div>

        <!-- Step 2: Hand Cards -->
        {#if trumpCardSelected}
          <div class="rounded-lg border border-zinc-800 bg-zinc-950/30 p-4">
            <div class="flex items-center gap-2 mb-3">
              <span class="flex items-center justify-center w-6 h-6 rounded-full {manualHand.length === 6 ? 'bg-green-600' : 'bg-indigo-600'} text-xs font-bold">2</span>
              <span class="text-sm font-medium text-zinc-300">Select your 6 hand cards</span>
              <span class="text-xs text-zinc-500 ml-auto">{manualHand.length}/6</span>
            </div>
            <div class="flex flex-wrap gap-2 min-h-[4rem] p-2 rounded border border-zinc-800 bg-zinc-950/50">
              {#each sortCards(manualHand) as c (cardKey(c))}
                <Card card={c} size="sm" selectable on:click={() => manualHand = manualHand.filter(x => cardKey(x) !== cardKey(c))} />
              {/each}
              {#if !manualHand.length}<div class="text-xs text-zinc-500 self-center">Click cards below to add.</div>{/if}
            </div>
            {#if manualHand.length < 6}
              <div class="mt-3">
                <div class="text-xs text-zinc-400 mb-2">Click to add ({6 - manualHand.length} more):</div>
                <div class="flex flex-wrap gap-1">
                  {#each getAvailableCards([...manualHand, currentTrumpCard]) as c (cardKey(c))}
                    <Card card={c} size="sm" selectable on:click={() => manualHand = [...manualHand, c]} />
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/if}

        <!-- Step 3: Opponent Trumps -->
        {#if manualHand.length === 6}
          <div class="rounded-lg border border-zinc-800 bg-zinc-950/30 p-4">
            <div class="flex items-center gap-2 mb-3">
              <span class="flex items-center justify-center w-6 h-6 rounded-full {step3Complete ? 'bg-green-600' : 'bg-indigo-600'} text-xs font-bold">3</span>
              <span class="text-sm font-medium text-zinc-300">Opponent's lowest trump <span class="text-amber-400">({suitSymbol(manualTrumpSuit)})</span></span>
            </div>
            <div class="text-xs text-zinc-500 mb-3">
              {#if lowestTrumpInHand}Your lowest: <span class="text-zinc-300 font-medium">{cardLabel(lowestTrumpInHand)}</span>{:else}You have no trumps{/if}
            </div>
            <div class="space-y-4">
              {#each Array(settings.numPlayers) as _, idx}
                {@const pid = `P${idx}`}
                {#if pid !== me}
                  {@const sel = opponentTrumps[pid]}
                  <div>
                    <div class="text-xs text-zinc-400 mb-2">{displayName(pid)}'s lowest ({suitSymbol(manualTrumpSuit)}):</div>
                    <div class="flex flex-wrap items-center gap-2">
                      <button class="rounded-md border px-3 py-1.5 text-xs font-medium {sel === 'None' ? 'border-amber-500 bg-amber-500/20 text-amber-300' : 'border-zinc-700 bg-zinc-950 text-zinc-400 hover:bg-zinc-900'}" on:click={() => clearOpponentTrump(pid)}>No Trump</button>
                      {#if sel && sel !== 'None'}
                        <div class="flex items-center gap-2 ml-2 px-2 py-1 rounded border border-indigo-600/50 bg-indigo-950/30">
                          <Card card={{ suit: manualTrumpSuit, rank: sel }} size="sm" />
                          <button class="text-xs text-zinc-400 hover:text-zinc-200" on:click={() => clearOpponentTrump(pid)}>✕</button>
                        </div>
                      {:else}
                        {#each remainingTrumpCards as c (cardKey(c))}
                          <button on:click={() => setOpponentTrump(pid, c.rank)}><Card card={c} size="sm" selectable /></button>
                        {/each}
                      {/if}
                    </div>
                  </div>
                {/if}
              {/each}
            </div>
          </div>

          {#if step3Complete}
            <div class="flex items-center justify-between p-4 rounded-lg border border-zinc-800 bg-zinc-950/30">
              <div>
                <div class="text-sm font-medium {manualStarts === me ? 'text-indigo-400' : 'text-zinc-400'}">{manualStarts === me ? 'You start!' : `${displayName(manualStarts)} starts`}</div>
                <div class="text-xs text-zinc-500">(By lowest trump)</div>
              </div>
              <button class="rounded-md bg-indigo-600 px-6 py-2.5 text-sm font-medium hover:bg-indigo-500" on:click={newGame}>Start Game</button>
            </div>
          {/if}
        {/if}
      </div>
    {/if}

    <!-- Main Game Area -->
    <div class="mt-6 grid gap-4 lg:grid-cols-3">
      <div class="lg:col-span-2 rounded-xl border border-zinc-800 bg-zinc-900/40 p-5">
        {#if state && (settings.computerShuffle || manualGameStarted)}
          <!-- Game Over Banner -->
          {@const loser = getLoser(state)}
          {#if loser}
            {@const youLost = loser === me}
            <div class="mb-4 p-4 rounded-lg border-2 {youLost ? 'border-zinc-500 bg-zinc-950/40' : 'border-indigo-500 bg-indigo-950/40'}">
              <div class="text-lg font-bold {youLost ? 'text-zinc-300' : 'text-indigo-400'}">
                {#if youLost}You're the Durak!{:else}{displayName(loser)} is the Durak!{/if}
              </div>
            </div>
          {/if}

          <!-- Opponents -->
          {#each Array(state.hands?.length || settings.numPlayers) as _, idx}
            {@const pid = `P${idx}`}
            {#if pid !== me}
              {@const hand = state.hands?.[idx] || []}
              {@const handKnown = getKnownCards(hand)}
              {@const handAll = hand.map(parseCard).filter(c => c !== null)}
              {@const handUnknownCount = getUnknownCount(hand)}
              {@const totalCards = hand.length}
              {@const pType = settings.computerShuffle ? settings.playerTypes[pid] : PlayerType.Real}
              {@const role = getPlayerRole(state, pid)}
              {@const isTurn = isPlayersTurn(state, pid)}
              <div class="mt-5 rounded-lg border {isTurn ? 'border-yellow-500/50' : 'border-zinc-800'} bg-zinc-950/30 p-4">
                <div class="flex items-center justify-between">
                  <div class="flex items-center gap-2">
                    <span class="text-xs {isTurn ? 'text-yellow-300 font-medium' : 'text-zinc-400'}">{displayName(pid)}</span>
                    {#if settings.computerShuffle}
                      <span class="text-[10px] px-1.5 py-0.5 rounded {pType === PlayerType.MCTS ? 'bg-indigo-900/50 text-indigo-300' : 'bg-zinc-800 text-zinc-400'}">{pType}</span>
                    {/if}
                    {#if role}
                      <span class="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-300">{role}</span>
                    {/if}
                    {#if handKnown.length > 0 || areAllCardsDeducible(state)}
                      <span class="text-[10px] text-amber-400">({areAllCardsDeducible(state) ? 'all known' : `${handKnown.length} known`})</span>
                    {/if}
                  </div>
                  <div class="flex items-center gap-2">
                    {#if settings.computerShuffle}
                      <label class="flex items-center gap-1 text-[10px] text-zinc-500 cursor-pointer">
                        <input type="checkbox" class="rounded w-3 h-3" bind:checked={showOpponentCards} /> Show
                      </label>
                    {/if}
                    <span class="text-xs text-zinc-500">{totalCards} cards</span>
                  </div>
                </div>
                <div class="mt-2 flex flex-wrap gap-2">
                  {#if (showOpponentCards || areAllCardsDeducible(state)) && settings.computerShuffle}
                    {#each sortCards(handAll) as c, i (cardKey(c) + '-' + i)}<Card card={c} size="sm" />{/each}
                  {:else}
                    {#each sortCards(handKnown) as c, i (cardKey(c) + '-' + i)}<Card card={c} size="sm" known />{/each}
                    {#each Array(handUnknownCount) as _, i (i)}<Card faceDown size="sm" />{/each}
                  {/if}
                  {#if !totalCards}<div class="text-xs text-zinc-500">No cards</div>{/if}
                </div>
              </div>
            {/if}
          {/each}

          <!-- Table & Stock -->
          <div class="mt-4 flex flex-col sm:flex-row sm:items-center gap-4">
            <div class="flex-1 rounded-lg border border-zinc-800 bg-zinc-950/30 p-4">
              <div class="flex items-center justify-between text-xs text-zinc-400">
                <span>Table</span>
                <div class="flex items-center gap-2">
                  <span class="px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-300">{state.phase}</span>
                  <span class="text-zinc-500">{actor(state) === me ? 'Your turn' : `${displayName(actor(state))}'s turn`}</span>
                </div>
              </div>
              <div class="mt-3 flex flex-wrap gap-3">
                {#each state.table as p, i (i)}
                  <div class="relative">
                    <Card card={parseCard(p.attack)} size="md" />
                    {#if p.defense}<div class="absolute left-6 top-6"><Card card={parseCard(p.defense)} size="md" /></div>{/if}
                  </div>
                {/each}
                {#if !state.table.length}<div class="text-xs text-zinc-500">No cards on table.</div>{/if}
              </div>
            </div>
            {#if true}
              {@const stockCount = state.stock?.length || 0}
              {@const trumpCard = stockCount > 0 ? parseCard(state.stock[0]) : null}
              <div class="flex-shrink-0 flex flex-col items-center justify-center gap-2 sm:w-24">
                <div class="text-xs text-zinc-400">Trump</div>
                {#if trumpCard}<Card card={trumpCard} size="sm" />{:else}<div class="text-3xl">{suitSymbol(state.trump)}</div>{/if}
                <div class="text-xs text-zinc-500 text-center">{stockCount > 0 ? `${stockCount} in stock` : 'Stock empty'}</div>
              </div>
            {/if}
          </div>

          <!-- Card Drawing UI (manual mode only) -->
          {#if !settings.computerShuffle && myUnknownCount(state) > 0}
            {@const unknownCount = myUnknownCount(state)}
            {@const usedCards = getUsedCardsInGame(state)}
            {@const availableCards = getAvailableCards(usedCards)}
            <div class="mt-4 rounded-lg border-2 border-amber-500/50 bg-amber-950/20 p-4">
              <div class="text-sm font-medium text-amber-300 mb-2">
                You drew {unknownCount} card{unknownCount > 1 ? 's' : ''} from the stock
              </div>
              <div class="text-xs text-zinc-400 mb-3">Select which card{unknownCount > 1 ? 's' : ''} you drew:</div>
              <div class="flex flex-wrap gap-1.5 max-h-48 overflow-y-auto">
                {#each availableCards as c (cardKey(c))}
                  <Card card={c} size="sm" selectable on:click={() => replaceUnknownCard(c)} />
                {/each}
              </div>
            </div>
          {/if}

          <!-- Your Hand -->
          {#if true}
            {@const myType = settings.computerShuffle ? settings.playerTypes[me] : PlayerType.Real}
            {@const myKnownToOpponents = (state?.hands?.[0] || []).filter(c => c?.type === 'public' || c?.Public).map(c => c?.type === 'public' ? { suit: c.suit, rank: c.rank } : c.Public)}
            {@const myRole = getPlayerRole(state, me)}
            {@const isMyTurn = isPlayersTurn(state, me)}
            {@const hasUnknownCards = !settings.computerShuffle && myUnknownCount(state) > 0}
            <div class="durak-felt mt-4 rounded-lg border {hasUnknownCards ? 'border-amber-500/50' : isMyTurn ? 'border-yellow-500/50' : 'border-zinc-800'} p-4">
            <div class="flex items-center justify-between text-xs text-zinc-400">
              <div class="flex items-center gap-3">
                <span class="{isMyTurn ? 'text-yellow-300 font-medium' : ''}">Your Hand</span>
                {#if settings.computerShuffle}
                  <span class="text-[10px] px-1.5 py-0.5 rounded {myType === PlayerType.Human ? 'bg-indigo-900/50 text-indigo-300' : 'bg-zinc-800 text-zinc-400'}">{myType}</span>
                {/if}
                {#if myRole}
                  <span class="text-[10px] px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-300">{myRole}</span>
                {/if}
                {#if hasUnknownCards}
                  <span class="text-[10px] px-1.5 py-0.5 rounded bg-amber-900/50 text-amber-300">{myUnknownCount(state)} unknown</span>
                {:else if areAllCardsDeducible(state) || myKnownToOpponents.length}
                  <span class="text-[10px] text-amber-400"><span class="inline-block w-3 h-3 bg-amber-500/80 text-white text-[7px] text-center rounded-sm mr-0.5">!</span>= known</span>
                {/if}
              </div>
              <span class="text-zinc-500">{myKnownHand(state).length}{hasUnknownCards ? ` + ${myUnknownCount(state)} unknown` : ''} cards</span>
            </div>
            <div class="mt-2 flex flex-wrap gap-2">
              {#each myKnownHand(state) as c (cardKey(c))}
                {@const playable = isCardPlayable(c) && !hasUnknownCards}
                {@const revealed = isCardKnownToOpponents(state, c) || areAllCardsDeducible(state)}
                <div class={playable ? '' : 'opacity-40'}>
                  <Card card={c} selectable={actor(state) === me && playable && !isGameOver(state)} disabled={actor(state) !== me || !playable || isGameOver(state) || hasUnknownCards} selected={bestActionForCard(c)} {revealed} on:click={() => playCard(c)} />
                </div>
              {/each}
              {#if hasUnknownCards}
                {#each Array(myUnknownCount(state)) as _, i (i)}
                  <Card faceDown size="md" />
                {/each}
              {/if}
            </div>
          </div>
          {/if}

          <!-- Non-card actions below hand -->
          {#if actor(state) === me}
            <div class="mt-4 flex flex-wrap gap-2">
              {#each legal.filter(a => !actionCard(a)) as a (actionKey(a))}
                <button class="rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-xs hover:bg-zinc-900" on:click={() => playAction(a)}>{actionText(a)}</button>
              {/each}
            </div>
          {/if}
        {:else if settings.computerShuffle && !state}
          <!-- Start Game screen with game rule settings -->
          <div class="flex flex-col items-center justify-center py-8">
            <div class="w-full max-w-sm space-y-4 mb-6">
              <!-- Deck Size -->
              <label class="block">
                <span class="text-sm font-medium text-zinc-300">Deck Size</span>
                <select
                  class="mt-1 w-full rounded-md border border-zinc-700 bg-zinc-950 px-3 py-2 text-sm"
                  value={settings.deckSize}
                  on:change={e => { settings = { ...settings, deckSize: parseInt(e.target.value) }; saveSettings(settings) }}
                >
                  <option value={32}>32 cards (7-Ace)</option>
                  <option value={36}>36 cards (6-Ace)</option>
                  <option value={40}>40 cards (5-Ace)</option>
                  <option value={44}>44 cards (4-Ace)</option>
                  <option value={48}>48 cards (3-Ace)</option>
                  <option value={52}>52 cards (2-Ace)</option>
                </select>
              </label>

              <!-- Trump Reflecting -->
              <label class="flex items-center gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  class="h-4 w-4 rounded border-zinc-700 bg-zinc-950 text-indigo-600"
                  checked={settings.trumpReflecting}
                  on:change={e => { settings = { ...settings, trumpReflecting: e.target.checked }; saveSettings(settings) }}
                />
                <div>
                  <span class="text-sm font-medium text-zinc-300">Trump Reflecting</span>
                  <p class="text-xs text-zinc-500">Show trump of same rank to redirect attack</p>
                </div>
              </label>

              <!-- Reflecting -->
              <label class="flex items-center gap-3 cursor-pointer">
                <input
                  type="checkbox"
                  class="h-4 w-4 rounded border-zinc-700 bg-zinc-950 text-indigo-600"
                  checked={settings.reflecting}
                  on:change={e => { settings = { ...settings, reflecting: e.target.checked }; saveSettings(settings) }}
                />
                <div>
                  <span class="text-sm font-medium text-zinc-300">Reflecting</span>
                  <p class="text-xs text-zinc-500">Play card of same rank to redirect attack</p>
                </div>
              </label>
            </div>

            <button class="rounded-md bg-indigo-600 px-8 py-3 text-lg font-medium hover:bg-indigo-500" on:click={newGame}>Start Game</button>
          </div>
        {:else}
          <div class="text-sm text-zinc-400">Loading…</div>
        {/if}
      </div>

      <!-- Right Panel: Actions/Solver -->
      <div class="rounded-xl border border-zinc-800 bg-zinc-900/40 p-5">
        {#if state && (settings.computerShuffle || manualGameStarted)}
          {@const currentActor = actor(state)}
          {@const isMyTurn = currentActor === me}
          {@const currentType = isMyTurn ? PlayerType.Human : (settings.computerShuffle ? settings.playerTypes[currentActor] : PlayerType.Real)}

          <!-- Header with turn info and hints toggle -->
          <div class="flex items-center gap-2 mb-3">
            <div class="text-sm font-medium">
              {isMyTurn ? 'Your' : `${displayName(currentActor)}'s`} Turn
              <span class="text-[10px] ml-1 px-1.5 py-0.5 rounded {currentType === PlayerType.MCTS ? 'bg-indigo-900/50 text-indigo-300' : 'bg-zinc-800 text-zinc-400'}">{currentType}</span>
            </div>
            <div class="flex-1"></div>
            {#if isMyTurn || (!settings.computerShuffle && currentActor === me)}
              <button
                class="rounded-md px-3 py-1.5 text-xs font-medium {settings.showHints ? 'bg-indigo-600 hover:bg-indigo-500' : 'border border-zinc-700 bg-zinc-900 hover:bg-zinc-800'}"
                on:click={toggleHints}
              >
                {settings.showHints ? 'Hints On' : 'Hints Off'}
              </button>
            {/if}
          </div>

          <!-- Solver settings (when hints enabled and it's my turn) -->
          {#if settings.showHints && isMyTurn}
            <div class="mb-3 flex flex-wrap gap-4 text-xs text-zinc-400">
              <div class="flex flex-col gap-1">
                <label class="flex items-center gap-1"><span class="w-24">Determinizations:</span><input class="w-20 rounded border border-zinc-700 bg-zinc-950 px-1.5 py-0.5 text-xs" type="number" min="1" value={settings.hintDeterminizations}
                  on:change={e => { settings = { ...settings, hintDeterminizations: parseInt(e.target.value) || 50 }; saveSettings(settings) }} /></label>
                <label class="flex items-center gap-1"><span class="w-24">Rollouts:</span><input class="w-20 rounded border border-zinc-700 bg-zinc-950 px-1.5 py-0.5 text-xs" type="number" min="100" value={settings.hintRollouts}
                  on:change={e => { settings = { ...settings, hintRollouts: parseInt(e.target.value) || 5000 }; saveSettings(settings) }} /></label>
              </div>
              <div class="flex flex-col gap-1 text-zinc-500"><span>% = win rate</span><span># = simulations</span><span class="text-zinc-600 italic text-[10px]">Tip: play with these numbers</span></div>
            </div>
          {/if}

          <!-- Action Display -->
          {#if !settings.computerShuffle && myUnknownCount(state) > 0}
            <div class="text-sm text-amber-400">
              Please specify which cards you drew before continuing.
            </div>
          {:else if isMyTurn}
            {@const actions = aggregate?.actions || legal.map(a => ({ action: a, mean_score: null }))}
            {@const { cardActions, otherActions } = getActionGroups(actions)}

            {#if cardActions.length}
              <div class="flex flex-wrap gap-2 mb-3">
                {#each cardActions as a (actionKey(a.action))}
                  {@const card = parseCard(a.action.card)}
                  {@const isBest = aggregate?.actions?.length && actionKey(a.action) === actionKey(aggregate.actions[0].action) && a.score != null}
                  {@const label = { attack: 'Attack', defend: 'Defend', throw: 'Throw', reflect: 'Reflect', reflect_trump: 'Show' }[a.action.type]}
                  {@const winPct = a.score != null ? (a.score * 100).toFixed(1) : null}
                  <button class="flex flex-col items-center gap-1 p-1.5 rounded-lg border {isBest ? 'border-indigo-500 bg-indigo-950/40' : 'border-zinc-700 bg-zinc-900/50'} hover:bg-zinc-800/50"
                    on:click={() => applyAnyAction(a.action, true)} title={winPct !== null ? `${winPct}% from ${a.visits}` : label}>
                    <div class="text-[10px] text-zinc-400">{label}</div>
                    <Card {card} size="sm" />
                    {#if winPct !== null}
                      <div class="flex flex-col items-center gap-0.5">
                        <div class="text-[10px] {a.score > 0.5 ? 'text-green-400' : a.score < 0.5 ? 'text-red-400' : 'text-zinc-400'}">{winPct}%</div>
                        <div class="w-10 h-1 bg-zinc-700 rounded-full overflow-hidden"><div class="h-full {a.score > 0.5 ? 'bg-green-500' : a.score < 0.5 ? 'bg-red-500' : 'bg-zinc-500'}" style="width: {winPct}%"></div></div>
                        <div class="text-[8px] text-zinc-500">{a.visits}</div>
                      </div>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}

            {#if otherActions.length}
              <div class="flex gap-2 mb-3">
                {#each otherActions as a (actionKey(a.action))}
                  {@const isBest = aggregate?.actions?.length && actionKey(a.action) === actionKey(aggregate.actions[0].action) && a.score != null}
                  {@const winPct = a.score != null ? (a.score * 100).toFixed(1) : null}
                  <button class="flex flex-col items-center gap-1 px-4 py-2 rounded-lg border {isBest ? 'border-indigo-500 bg-indigo-950/40' : 'border-zinc-700 bg-zinc-900/50'} hover:bg-zinc-800/50" on:click={() => applyAnyAction(a.action, true)}>
                    <div class="text-xs font-medium">{actionText(a.action)}</div>
                    {#if winPct !== null}
                      <div class="text-[10px] {a.score > 0.5 ? 'text-green-400' : a.score < 0.5 ? 'text-red-400' : 'text-zinc-400'}">{winPct}%</div>
                      <div class="text-[8px] text-zinc-500">{a.visits}</div>
                    {/if}
                  </button>
                {/each}
              </div>
            {/if}
          {:else if !settings.computerShuffle}
            <!-- Manual mode: opponent's turn - show action pickers -->
            {@const types = getActionsByType(legal)}
            {#if types.other.length}
              <div class="flex flex-wrap gap-2 mb-4">
                {#each types.other as a (actionKey(a))}
                  <button class="rounded-md border-2 px-4 py-2.5 text-sm font-semibold shadow-lg border-indigo-500 bg-indigo-950 text-indigo-200 hover:bg-indigo-900" on:click={() => applyAnyAction(a, true)}>{actionText(a)}</button>
                {/each}
              </div>
            {/if}
            {#each [['Attack', types.attack], ['Defend', types.defend], ['Throw', types.throw], ['Reflect', types.reflect], ['Show Trump', types.reflectTrump]] as [label, acts]}
              {#if acts.length}
                <div class="mb-3">
                  <div class="text-xs text-zinc-400 mb-1.5">{label}:</div>
                  <div class="flex flex-wrap gap-1.5">
                    {#each sortActionsByCard(acts) as a (actionKey(a))}
                      <Card card={actionCard(a)} size="sm" selectable on:click={() => applyAnyAction(a, true)} />
                    {/each}
                  </div>
                </div>
              {/if}
            {/each}
            <details class="text-xs">
              <summary class="text-zinc-500 cursor-pointer hover:text-zinc-300 mb-2">Manual card input</summary>
              <div class="space-y-2 mt-2 p-2 rounded border border-zinc-800 bg-zinc-950/50">
                <div class="grid grid-cols-2 gap-2">
                  <label class="grid gap-1"><span class="text-zinc-400">Suit</span><select class="rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5" bind:value={opponentPlaySuit}>{#each SUITS as s}<option value={s}>{s}</option>{/each}</select></label>
                  <label class="grid gap-1"><span class="text-zinc-400">Rank</span><select class="rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5" bind:value={opponentPlayRank}>{#each RANKS as r}<option value={r}>{r}</option>{/each}</select></label>
                </div>
                <button class="w-full rounded-md border border-zinc-700 bg-zinc-950 px-2 py-1.5 hover:bg-zinc-900" on:click={applyOpponentCard}>Apply</button>
              </div>
            </details>
          {:else if settings.computerShuffle && aiThinking && aiThinkingPlayer === currentActor}
            <!-- AI thinking indicator -->
            <div class="rounded-lg border border-indigo-700/50 bg-indigo-950/30 p-4">
              <div class="flex items-center gap-3">
                <div class="animate-spin rounded-full h-5 w-5 border-2 border-indigo-500 border-t-transparent"></div>
                <div>
                  <div class="text-sm font-medium text-indigo-300">{currentType === PlayerType.MCTS ? 'Thinking...' : 'Playing...'}</div>
                  <div class="text-xs text-zinc-500">{currentType === PlayerType.Random ? 'Random' : 'MCTS'}</div>
                </div>
              </div>
            </div>
          {:else}
            <div class="text-xs text-zinc-500">Waiting...</div>
          {/if}
        {/if}
      </div>
    </div>
  </div>
  </main>

  <footer class="py-2 text-center text-sm text-zinc-500">
    <p>&copy; {year} - <a href="https://jorisperrenet.github.io" class="hover:text-zinc-300 underline">Joris Perrenet</a></p>
  </footer>
</div>

