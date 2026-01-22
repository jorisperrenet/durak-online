/**
 * `durak-core` serializes cards as:
 *   { suit: "Clubs"|"Diamonds"|"Hearts"|"Spades", rank: "Six"|...|"Ace" }
 *
 * We map them to `web/public/cards/*_of_*.png` from playing-cards-assets.
 */

const suitToFilename = {
  Clubs: 'clubs',
  Diamonds: 'diamonds',
  Hearts: 'hearts',
  Spades: 'spades',
}

const rankToFilename = {
  Two: '2',
  Three: '3',
  Four: '4',
  Five: '5',
  Six: '6',
  Seven: '7',
  Eight: '8',
  Nine: '9',
  Ten: '10',
  Jack: 'jack',
  Queen: 'queen',
  King: 'king',
  Ace: 'ace',
}

const suitToSymbol = {
  Clubs: '♣',
  Diamonds: '♦',
  Hearts: '♥',
  Spades: '♠',
}

const rankToShort = {
  Two: '2',
  Three: '3',
  Four: '4',
  Five: '5',
  Six: '6',
  Seven: '7',
  Eight: '8',
  Nine: '9',
  Ten: '10',
  Jack: 'J',
  Queen: 'Q',
  King: 'K',
  Ace: 'A',
}

const BASE_URL = (import.meta?.env?.BASE_URL ?? '/').replace(/\/+$/, '/')

function withBase(path) {
  const p = String(path || '').replace(/^\/+/, '')
  return `${BASE_URL}${p}`
}

export function cardFilename(card) {
  if (!card) return null
  const r = rankToFilename[card.rank]
  const s = suitToFilename[card.suit]
  if (!r || !s) return null
  return `${r}_of_${s}.png`
}

export function cardBackUrl() {
  return withBase('cards/back.png')
}

export function cardImageUrl(card) {
  const fn = cardFilename(card)
  if (!fn) return cardBackUrl()
  return withBase(`cards/${fn}`)
}

export function suitSymbol(suit) {
  return suitToSymbol[suit] || '?'
}

export function cardLabel(card) {
  if (!card) return ''
  return `${rankToShort[card.rank] || card.rank}${suitSymbol(card.suit)}`
}

export function isRedSuit(suit) {
  return suit === 'Diamonds' || suit === 'Hearts'
}
