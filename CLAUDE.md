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

This is a Rust implementation of the board game **Order and Chaos** — a 6x6 variant of tic-tac-toe. The project is in early development (currently just a stub `main.rs`).

**Game rules:**
- 6x6 grid, played with X's and O's
- Either player may place either symbol on any open square each turn
- **Order** wins by getting *exactly* five in a row (vertically, horizontally, or diagonally) — six-in-a-row does NOT count
- **Chaos** wins by filling the board without five in a row

**Planned features** (per README):
- Two-player and single-player modes
- AI opponent using alpha-beta pruning
- Move input format: coordinate + piece, e.g. `j2x` to place X at column j, row 2

## Architecture

The game is a single binary (`orderchaos`). No external dependencies. All logic lives in `src/`.

## Extra Instructions
- There should be full unit test coverage. Whenever functions are changed or added, tests should be updated as well.