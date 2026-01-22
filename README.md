# Durak-online, a remodel of my previous [durak](https://github.com/jorisperrenet/durak)

A browser-based implementation of [MCTS](https://en.wikipedia.org/wiki/Monte_Carlo_tree_search) to guide gameplay in the card game [Durak](https://en.wikipedia.org/wiki/Durak), with a clean web UI.

## Two Modes of Play

Toggle **Computer Shuffle** in the header to switch between modes:

- **Computer Shuffle ON** — Play against AI opponents. The computer deals the cards and you play against MCTS or random players. *Toggle for real game help.*
- **Computer Shuffle OFF** — Get help in a real-life game. Input the cards you see and the actions opponents take, and the AI suggests your optimal moves. *Toggle to play vs computer.*

## Game Rules

This implements traditional Durak with optional reflecting rules:

- Support for 2-6 players and deck sizes from 32 to 52 cards
- The attacker can only attack with one card at a time (irrelevant for gameplay if you think about it)
- **Reflecting**: Defender can pass the attack to the next player by playing a card of the same rank
- **Trump reflecting**: Defender can divert an attack by showing (not playing) a trump of the same rank. This can only be done once per trump per trick, and only if no card is defended yet.
- The starting player is determined by who has the lowest trump

## Types of Players

| Type | Description |
| ---- | ----------- |
| Human | Interactive gameplay through the UI |
| Random | Performs a random legal action |
| MCTS | Multi-threaded Determinized MCTS — samples possible card distributions and runs Monte Carlo tree search |

## How to Win in a Real-Life Game

1. Set **Computer Shuffle** to OFF (the default)
2. Enter the trump card and your 6 starting cards
3. Indicate each opponent's lowest trump (determines who starts)
4. As the game progresses, select the actions opponents take
5. When it's your turn, the AI shows win probabilities for each possible move
6. Once the stock is empty and cards become deducible, opponent hands are revealed automatically

Configure **Determinizations** (number of random card distributions to sample) and **Rollouts** (MCTS simulations per distribution) to balance accuracy vs speed. Higher values = better AI but slower computation.

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/) with the `wasm32-unknown-unknown` target
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)

```bash
# Install wasm32 target (if using rustup)
rustup target add wasm32-unknown-unknown

# Install wasm-pack
cargo install wasm-pack
```

### Build & Run

```bash
# 1. Build the WASM module
cd durak-wasm
wasm-pack build --target web
cp pkg/durak_wasm* ../web/src/wasm/

# 2. Start the web app
cd ../web
npm install
npm run dev
```

The app will be available at `http://localhost:5173`.

### Production Build/Deployment

```bash
cd web
npm run build
# OR
npm run deploy
```

## Project Structure

```
durak3/
├── durak-core/       # Rust game engine (rules, state, MCTS)
├── durak-wasm/       # WebAssembly bindings
└── web/              # Svelte frontend
    └── src/
        ├── App.svelte           # Main UI
        ├── wasm/                # WASM module (generated)
        └── workers/             # Web Workers for parallel MCTS
```
