# AGENTS.md — PokéDash Advance Agentic Guidelines

Welcome to **PokéDash Advance**! This document provides operational directives, architecture standards, conventions, and guidelines for AI coding agents (and human contributors) working on this codebase.

---

## 1. Project Philosophy & Core Stack

- **Concept**: A retro Game Boy Advance / LCD handheld style Pokémon hunting, catching, battling, and collecting web game.
- **Backend**: Rust (Edition 2021) using **Axum 0.7**, **Tokio**, **Tower/Tower-HTTP**, and **Serde**.
- **Frontend**: Lightweight single-page web UI in [`static/index.html`](static/index.html), featuring custom Base64-embedded fonts (`PixelEmulator-xq08.ttf`), responsive GBA console skin, full keyboard controls, and an inline/referenced 37-icon retro pixel SVG sprite sheet ([`static/pixel-icons.svg`](static/pixel-icons.svg)).
- **Authentication & Security**:
  - Primary: **Supabase GoTrue Email Authentication** (`/auth/v1`) with signed Bearer JWT sessions.
  - Trainer Handles: Case-insensitive unique `playerName` constraint (`idx_players_lower_username`).
  - Strict Authorization: All gameplay endpoints strictly gated via Axum [`AuthenticatedTrainer`](src/server.rs) extractor.
- **Database & Persistence**:
  - Primary: **Supabase PostgREST / PostgreSQL cloud database** ([`src/supabase.rs`](src/supabase.rs)).
  - Multiplayer: Real-time global leaderboards indexed by `idx_players_xp_desc`.
  - Offline Fallback / Local Cache: JSON files in `~/.pokedash/` (`player_<username>.json`, `pokedex_<username>.json`, etc.) ([`src/storage.rs`](src/storage.rs)).
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
│   ├── server.rs               # Axum REST API, AuthenticatedTrainer extractor & routing
│   ├── service.rs              # Core gameplay logic (catch, battle, heal, store, scoped by user)
│   ├── game.rs                 # Game math (formulas, catch rates, arena divisions)
│   ├── models.rs               # Strongly typed Serde models (Player, Auth, Leaderboard)
│   ├── storage.rs              # Multi-tenant hybrid persistence (Supabase Cloud + Local JSON)
│   └── supabase.rs             # Supabase GoTrue Auth client, PostgREST ops & token caching
├── static/
│   ├── index.html              # Main GBA web UI, Auth Modal, game loop, audio, canvas & tabs
│   ├── keyboard.html           # Standalone hardware & keyboard testing interface
│   ├── pixel-icons.svg         # Crisp 8-bit/16-bit SVG pixel art symbol sheet
│   ├── fonts/                  # PixelEmulator and Player2 TTF/OTF font assets
│   └── sprites/                # Local sprite cache fallback
├── tools/                      # Automation & code generation scripts (Python)
├── AGENTS.md                   # This file (Agent directives & guidelines)
├── CONTEXT.md                  # Comprehensive architectural and gameplay context
├── ARCHITECTURE.md             # Detailed architecture, security & concurrency flows
├── Cargo.toml                  # Rust crate dependencies and build configuration
└── README.md                   # Player documentation & quickstart
```

---

## 3. Agent Operating Directives

When modifying this repository, every AI agent must strictly follow these rules:

### A. Authentication & Strict Access Control
- **No Unauthenticated Gameplay**: Never bypass or remove `AuthenticatedTrainer` from gameplay routes (`/api/player`, `/api/hunt`, `/api/wild`, `/api/catch`, `/api/battle/*`, `/api/party/*`, `/api/arena/*`, `/api/store/*`, `/api/pokedex`, `/api/catalog`). Requests without a valid Bearer token must return `401 Unauthorized`.
- **Default Email Auth**: Authentication uses Supabase email + password, supporting 6-digit confirmation codes (`/api/auth/verify`) and direct confirmation links (`/#access_token=...`).
- **Unique Trainer Handles**: Trainer usernames must remain unique and case-insensitive. Never remove or circumvent `idx_players_lower_username` or `is_username_taken`.
- **In-Memory Token Cache**: In `SupabaseClient`, preserve the 10-minute in-memory JWT cache (`token_cache`) to avoid round-tripping to Supabase Auth on every user action.

### B. Multiplayer & Data Scoping
- **Multi-Tenant Scoping**: All storage and service operations must receive and isolate by `username` (`player_<username>.json`, `pokedex_<username>.json`). Never revert to hardcoded or global single-user storage paths.
- **Leaderboard Performance**: The global leaderboard (`GET /api/leaderboard`) queries `public.players` ordered by `xp DESC`. Maintain `idx_players_xp_desc` index integrity.

### C. Preserving Retro Aesthetic & Pixel Design
- **No Unicode Emojis**: Emojis (e.g., 🌾, ⚔️, 💖, 🏆) are strictly forbidden in UI templates and gameplay messages.
- **Always Use Pixel Icons**: Render icons using `<svg class="pixel-icon"><use href="#px-<name>"></use></svg>` or the JavaScript helper `pIcon('<name>')`.
- **Pixel Crisp Rendering**: Maintain CSS `image-rendering: pixelated;` and SVG `shape-rendering: crispEdges;` for all sprites, badges, and icons.
- **Hardware-First Styling**: The interface simulates an authentic GBA / Game Boy handheld console with tactile D-Pad buttons, A/B circles, and top slide switches (SFX, CRT, FULL).

### D. Rust Code Integrity & Architecture
- **Async Safety**: Axum routes are async (`tokio`). When the synchronous service layer bridges into async Supabase calls, use `tokio::task::block_in_place` or ensure non-blocking patterns.
- **Fail-Safe Persistence**: Never panic or break gameplay if Supabase is offline or network fails. All storage operations in [`src/storage.rs`](src/storage.rs) must fall back to local disk (`~/.pokedash/`).
- **Strict Typing**: All game payloads and database entities must map to serializable structs in [`src/models.rs`](src/models.rs). Do not pass raw untyped JSON strings between layers. Always provide `#[serde(default)]` on model fields to handle partial records gracefully.
- **Compilation Verification**: Always verify code changes with `cargo check` and `cargo test` (or `cargo build`) before considering a task complete.

### E. Frontend Conventions (`static/index.html`)
- **Single-File Architecture**: The game UI is delivered as a zero-build standalone HTML/CSS/JS file to guarantee instant execution via `./target/release/pokedash`.
- **Input Lockdown**: When unauthenticated or when `#auth-modal` is visible, all GBA controls and keyboard hotkeys must be blocked.
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

### Testing Auth Endpoints
```bash
# Test public status
curl -s http://localhost:8080/api/status

# Test unauthenticated rejection (Must return 401)
curl -s -i http://localhost:8080/api/player

# Test sign in
curl -s -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"trainer@pokemon.com","password":"password123"}'

# Test leaderboard
curl -s http://localhost:8080/api/leaderboard
```

### Icon Generation
Custom pixel icons are generated from ASCII grids into SVG symbols:
```bash
python3 tools/generate_pixel_icons.py
```
This updates [`static/pixel-icons.svg`](static/pixel-icons.svg).

---

## 5. Definition of Done for Agents

1. Code compiles cleanly with zero warnings or errors (`cargo check`).
2. Server boots and serves HTTP endpoints without regression (`GET /api/status`, `GET /api/leaderboard`).
3. Unauthenticated requests to gameplay endpoints are strictly rejected with `401 Unauthorized`.
4. Duplicate or invalid trainer usernames are rejected with `409 Conflict` / `400 Bad Request`.
5. Frontend UI remains responsive, visually aligned to the GBA theme, and free of browser console errors.
6. Changes are accurately described and committed to Git with conventional commit messages (e.g. `feat: ...`, `fix: ...`, `refactor: ...`).
