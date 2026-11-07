# AGENTS.md — PokéDash Advance Agentic Guidelines

Welcome to **PokéDash Advance**! This document provides operational directives, architecture standards, conventions, and guidelines for AI coding agents (and human contributors) working on this codebase.

---

## 1. Project Philosophy & Core Stack

- **Concept**: A retro Game Boy Advance / LCD handheld style Pokémon hunting, catching, battling, and collecting web game.
- **Backend**: Rust (Edition 2021) using **Axum 0.7**, **Tokio**, **Tower/Tower-HTTP**, and **Serde**.
- **Frontend**: Lightweight single-page web UI in [`static/index.html`](static/index.html), featuring custom Base64-embedded fonts (`PixelEmulator-xq08.ttf`), responsive GBA console skin, full keyboard controls, and an inline/referenced 37-icon retro pixel SVG sprite sheet ([`static/pixel-icons.svg`](static/pixel-icons.svg)).
- **Database & Persistence**:
  - Primary: **Supabase PostgREST / PostgreSQL cloud database** ([`src/supabase.rs`](src/supabase.rs)).
  - Offline Fallback / Local Cache: JSON files in `~/.pokedash/` (`player.json`, `pokedex.json`, `wild_pokemon.json`, `arena_battle.json`) ([`src/storage.rs`](src/storage.rs)).
- **Pokémon Dataset**: Pre-compiled JSON dictionary with 1025 Pokémon species, stats, abilities, types, and rarity weights ([`data/pokemon_data.json`](data/pokemon_data.json)).

---

## 2. Directory Layout & Key Modules

```
pokedash/
├── .agents/                    # Agentic configurations, skills & templates
├── .gemini/                    # Gemini workspace skills & directives
├── data/
│   └── pokemon_data.json       # All 1025 Gen I-IX Pokémon stats, types & rarity
├── src/
│   ├── main.rs                 # Server entry point & runtime initialization
│   ├── lib.rs                  # Library crate root exposing internal modules
│   ├── cli.rs                  # Clap CLI arguments, ASCII banner & runner
│   ├── server.rs               # Axum REST API endpoints, CORS & routing
│   ├── service.rs              # Core gameplay logic (catch, battle, heal, store)
│   ├── game.rs                 # Game math (formulas, catch rates, arena divisions)
│   ├── models.rs               # Strongly typed Serde models & game states
│   ├── storage.rs              # Hybrid persistence (Supabase Cloud + Local JSON)
│   └── supabase.rs             # Supabase HTTP client, auto-migration & PostgREST ops
├── static/
│   ├── index.html              # Main GBA web UI, game loop, audio, canvas & tabs
│   ├── keyboard.html           # Standalone hardware & keyboard testing interface
│   ├── pixel-icons.svg         # Crisp 8-bit/16-bit SVG pixel art symbol sheet
│   ├── fonts/                  # PixelEmulator and Player2 TTF/OTF font assets
│   └── sprites/                # Local sprite cache fallback
├── tools/                      # Automation & code generation scripts (Python)
├── AGENTS.md                   # This file (Agent directives & guidelines)
├── CONTEXT.md                  # Comprehensive architectural and gameplay context
├── Cargo.toml                  # Rust crate dependencies and build configuration
└── README.md                   # Player documentation & quickstart
```

---

## 3. Agent Operating Directives

When modifying this repository, every AI agent must strictly follow these rules:

### A. Preserving Retro Aesthetic & Pixel Design
- **No Unicode Emojis**: Emojis (e.g., 🌾, ⚔️, 💖, 🏆) are strictly forbidden in UI templates and gameplay messages.
- **Always Use Pixel Icons**: Render icons using `<svg class="pixel-icon"><use href="#px-<name>"></use></svg>` or the JavaScript helper `pIcon('<name>')`.
- **Pixel Crisp Rendering**: Maintain CSS `image-rendering: pixelated;` and SVG `shape-rendering: crispEdges;` for all sprites, badges, and icons.
- **Hardware-First Styling**: The interface simulates an authentic GBA / Game Boy handheld console with tactile D-Pad buttons, A/B circles, and top slide switches (SFX, CRT, FULL).

### B. Rust Code Integrity & Architecture
- **Async Safety**: Axum routes are async (`tokio`). When the synchronous service layer bridges into async Supabase calls, use `tokio::task::block_in_place` or ensure non-blocking patterns.
- **Fail-Safe Persistence**: Never panic or break gameplay if Supabase is offline or network fails. All storage operations in [`src/storage.rs`](src/storage.rs) must fall back to local disk (`~/.pokedash/`).
- **Strict Typing**: All game payloads and database entities must map to serializable structs in [`src/models.rs`](src/models.rs). Do not pass raw untyped JSON strings between layers.
- **Compilation Verification**: Always verify code changes with `cargo check` and `cargo test` (or `cargo build`) before considering a task complete.

### C. Frontend Conventions (`static/index.html`)
- **Single-File Architecture**: The game UI is currently delivered as a zero-build standalone HTML/CSS/JS file to guarantee instant execution via `./target/release/pokedash`.
- **Keyboard & Touch Parity**: Every action button has a keyboard shortcut hint (e.g. `[1] FIGHT`, `[2] BAG`, `[SPACE] SEARCH`, `[H] HEAL`, `[C] CRY`, `[W/A/S/D]` navigation). Maintain this dual-input capability.
- **Audio Synthesizer**: Uses Web Audio API procedural sound generation (`SFX.blip()`, `SFX.hit()`, `SFX.catchSuccess()`, `SFX.levelUp()`, `SFX.heal()`) alongside official PokeAPI mp3/ogg cries. Do not break procedural audio.

---

## 4. Common Developer Workflows

### Compiling and Running
```bash
# Fast compilation check
cargo check

# Run dev server on port 8080
cargo run -- --port 8080

# Build production binary
cargo build --release

# Run background daemon or test binary
./target/release/pokedash --port 8080
```

### Checking Supabase Connectivity
Environment variables in `.env` (or project root):
```bash
SUPABASE_URL="https://<project-ref>.supabase.co"
SUPABASE_KEY="<anon-key>"
```
The application auto-loads `.env` on boot via `supabase::load_dotenv()`.

### Icon Generation
Custom pixel icons are generated from ASCII grids into SVG symbols:
```bash
python3 tools/generate_pixel_icons.py
```
This updates [`static/pixel-icons.svg`](static/pixel-icons.svg).

---

## 5. Definition of Done for Agents

1. Code compiles cleanly with zero warnings or errors (`cargo check`).
2. Server boots and serves HTTP endpoints without regression (`GET /api/status`, `GET /api/player`).
3. Frontend UI remains responsive, visually aligned to the GBA theme, and free of browser console errors.
4. Changes are accurately described and committed to Git with conventional commit messages (e.g. `feat: ...`, `fix: ...`, `refactor: ...`).
