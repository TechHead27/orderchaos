# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build          # Build the project
cargo run            # Run the game
cargo test           # Run all tests
cargo test <name>    # Run a single test by name
cargo clippy         # Lint
```

## Project Overview

This is a Rust implementation of the board game **Order and Chaos** — a 6x6 variant of tic-tac-toe. The project currently consists of a main.rs file containing the input loop and a game folder containing the game logic.

**Game rules:**
- 6x6 grid, played with X's and O's
- Either player may place either symbol on any open square each turn
- **Order** wins by getting *exactly* five in a row (vertically, horizontally, or diagonally) — six-in-a-row does NOT count
- **Chaos** wins by filling the board without five in a row

**Planned features** (per README):
- Single-player mode against AI
- AI opponent using alpha-beta pruning

## Architecture

The game is a single binary (`orderchaos`). The external dependencies are downloaded and built by cargo. All logic lives in `src/`.

## Extra Instructions
- There should be full unit test coverage. Whenever functions are changed or added, tests should be updated as well.