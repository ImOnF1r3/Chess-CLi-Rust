# Chess-CLi-Rust

A basic Chess game for the terminal I wrote to practice Rust. It’s a work in progress, but it's functional and supports playing against the Stockfish engine.

## Features
* **CLI Board:** Simple rendering using ANSI colors.
* **AI:** Integration with Stockfish via pipes.
* **Logic:** Basic moves, Castling, En Passant, and Promotions.
* **FEN:** Support to import/export positions.

## Requirements
* **Stockfish:** Must be installed and in your PATH.
* **Nerd Font:** Required to display the piece icons correctly (e.g., JetBrainsMono Nerd Font).

## How to run
1. Clone the repo.
2. Run `cargo run`.

## Commands
* `e2e4` or `e2-e4`: Move a piece.
* `0-0` / `0-0-0`: Castling.
* `enemy on`: Enable Stockfish.
* `show fen`: Print current FEN.
* `help`: Show all commands.

---
Made by [ImOnF1r3](https://github.com/ImOnF1r3)
