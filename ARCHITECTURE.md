# ARCHITECTURE.md — PokéDash Engine Architecture & Data Flows

This document details the software architecture, design patterns, security model, and internal component lifecycles for developers and autonomous agents working on **PokéDash Advance**.

---

## 1. Modular Layering

The codebase is organized into five distinct architectural layers with unidirectional dependencies and strict separation of concerns:

```
┌─────────────────────────────────────────────────────────────┐
│                   Presentation Layer                        │
│   static/index.html (GBA LCD UI, Event Loop, Audio, Canvas) │
│   - Retro Auth Modal (Login, Signup, Verify, Resend)        │
│   - URL Hash Fragment Email Confirmation Processor          │
│   - Input Lockdown (Controls disabled while unauthenticated)│
└──────────────────────────────┬──────────────────────────────┘
                               │ HTTPS / JSON REST API (Bearer JWT)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Identity & Security Layer                 │
│   src/server.rs (AuthenticatedTrainer Axum Extractor)       │
│   - Bearer Token Validation & In-Memory TTL Cache (10 min)  │
│   - Case-Insensitive Unique Trainer Name Enforcement        │
│   - Rejection of Unauthenticated Game Requests (HTTP 401)   │
└──────────────────────────────┬──────────────────────────────┘
                               │ Authenticated Trainer Context
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Transport Layer                         │
│   src/server.rs (Axum Router, Handlers, CORS, Assets)       │
│   - Protected Endpoints: /api/player, /api/hunt, /api/catch │
│   - Public Endpoints:    /api/status, /api/leaderboard      │
└──────────────────────────────┬──────────────────────────────┘
                               │ Scoped by Trainer Username
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                      Domain Layer                           │
│   src/service.rs (GameService, Business Logic, Battles)     │
│   src/game.rs    (Catch/XP Math, Type Advantages, Arena)    │
│   src/models.rs  (Entities, Data Transfer Objects, Serde)   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Persistence Layer                         │
│   src/storage.rs  (Hybrid Storage: Cloud + Offline Cache)   │
│   src/supabase.rs (PostgREST Client, Table Schema, Indexes) │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Authentication, Identity & Access Control

### 2.1 Default Email Authentication (Supabase GoTrue)
- **Email as Primary Identity**: Authentication is built on top of Supabase Auth (`/auth/v1`).
- **Registration**:
  1. Client sends `POST /api/auth/signup` with `email`, `password`, and desired `username`.
  2. The server verifies that the username is 3–20 characters, alphanumeric (`_` and `-` permitted), and checks availability against `public.players` using a case-insensitive `ilike` query.
  3. If available, the user is registered in Supabase Auth with metadata `{"username": "<playerName>"}`.
  4. An initial player record is created in `public.players`.
  5. If email confirmation is enabled, Supabase dispatches a confirmation email containing a 6-digit OTP code or confirmation link.
- **Login**:
  - Client sends `POST /api/auth/login` with `email` and `password`.
  - On success, Supabase issues a signed JWT `access_token` and returns the trainer profile.
- **Email Confirmation & Direct Redirect**:
  - **Manual Code**: Trainers can enter the 6-digit code via `POST /api/auth/verify`.
  - **Link Redirect**: Clicking the confirmation email link redirects back to the game with an access token in the URL hash (`/#access_token=...&type=signup`). The frontend automatically extracts the token, queries `GET /api/auth/me`, clears the URL fragment, and logs the player in immediately.
  - **Resend**: Trainers can request a fresh code via `POST /api/auth/resend`.

### 2.2 Case-Insensitive Unique `playerName`
Player handles are globally unique and case-insensitive across the platform:
- **Database Level**: Enforced via a unique PostgreSQL functional index:
  ```sql
  CREATE UNIQUE INDEX IF NOT EXISTS idx_players_lower_username ON public.players (lower(username));
  ```
- **Application Level**: Pre-validated via `SupabaseClient::is_username_taken` using `username=ilike.<name>` before initiating Supabase Auth registration. Duplicate attempts return `HTTP 409 Conflict`.

### 2.3 Strict Authorization Gating (`AuthenticatedTrainer`)
- All gameplay routes strictly require the custom Axum extractor `AuthenticatedTrainer`:
  ```rust
  pub struct AuthenticatedTrainer {
      pub username: String,
      pub user_id: String,
      pub email: String,
  }
  ```
- **Token Verification**:
  1. Extracts `Authorization: Bearer <token>` from incoming HTTP headers.
  2. Checks an in-memory `RwLock<HashMap<String, (AuthUser, Instant)>>` cache (TTL: 10 minutes) for sub-millisecond route execution.
  3. If cache misses, validates the token against Supabase `/auth/v1/user`.
  4. Any missing, malformed, or expired token immediately aborts the pipeline with `HTTP 401 Unauthorized` (`{"code": "unauthorized" | "session_expired"}`).
- **UI Lockdown**:
  - The retro GBA frontend displays a persistent full-screen backdrop modal (`#auth-modal`) when unauthenticated.
  - All hardware buttons (D-Pad, A/B, Select, Start), keyboard hotkeys (`Space`, `1-4`, `W/A/S/D`, `Q/E`, `H`, `C`), and location tabs are intercepted and blocked until authenticated.

---

## 3. Multiplayer & Real-Time Leaderboards

PokéDash supports a shared multiplayer universe through centralized cloud persistence:

```
┌─────────────────────────────────────────────────────────────┐
│                  Global Players Table                       │
│  - username (PK, unique case-insensitive)                   │
│  - xp (Indexed DESC via idx_players_xp_desc)                │
│  - currency, stats, party, arena_badges                     │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                 GET /api/leaderboard                        │
│  Returns top 50 ranked trainers:                            │
│  1. Rank, Username, Level, XP, Gold                         │
│  2. Badges Count (out of 10)                                │
│  3. Unique Pokémon Caught (out of 1025)                     │
│  4. Active Party Lead Pokémon & Live Sprite                 │
└─────────────────────────────────────────────────────────────┘
```

- **Query Optimization**: A dedicated B-Tree index on `public.players (xp DESC)` allows instant ranking computation without full table scans.
- **Live Lead Pokémon**: Each leaderboard entry shows the player's active party lead Pokémon name, level, and sprite, reflecting live team updates.

---

## 4. In-Memory Concurrency & Synchronization

- **GameService State**: The core game state is guarded inside an `Arc<RwLock<()>>` inside [`src/service.rs`](src/service.rs):
  ```rust
  pub struct GameService {
      pub storage: Arc<Storage>,
      pub lock: Arc<RwLock<()>>,
  }
  ```
- **Player-Scoped Operations**: Every service method receives `&str username` extracted from the authenticated session:
  ```rust
  pub fn hunt(&self, username: &str) -> Result<WildPokemonEncounter, String>;
  pub fn catch(&self, username: &str, ball_input: &str) -> Result<CatchAttemptResult, String>;
  pub fn get_player(&self, username: &str) -> PlayerData;
  ```
- **Read Operations**: Endpoints like `GET /api/player`, `GET /api/wild`, and `GET /api/pokedex` acquire read locks (`self.lock.read().unwrap()`) for parallel reads.
- **Mutating Operations**: Operations like `POST /api/hunt`, `POST /api/catch`, and `POST /api/battle/wild-attack` acquire exclusive write locks (`self.lock.write().unwrap()`), mutate player state, persist changes through [`src/storage.rs`](src/storage.rs), and release the lock.

---

## 5. Storage & Fail-Safe Persistence Flow

The storage subsystem implements a **Write-Through / Read-Fallback** pattern scoped per trainer:

```
Save Operation:
1. Write to Supabase cloud table (players, pokedex, wild_encounters, arena_battles).
2. Concurrently or sequentially write to local disk (~/.pokedash/*_<username>.json).
3. If Supabase fails (offline / network error), log a warning and proceed with local save.

Load Operation:
1. If Supabase configured: attempt to read from cloud table for the authenticated username.
2. If remote record exists, sync/cache to local file and return.
3. If remote fails or is empty, read from local ~/.pokedash/*_<username>.json.
4. If local file exists, automatically push/migrate local record to Supabase.
```

---

## 6. Key Performance Optimizations

1. **In-Memory JWT Cache**: Token validation against Supabase Auth is cached in an `RwLock<HashMap>` for 10 minutes, avoiding redundant network round-trips on every gameplay action.
2. **Pre-Loaded Pokémon Catalog**: All 1025 Pokémon definitions are parsed into memory at startup as a `HashMap<u32, Pokemon>` and indexed into `rarity_pools: HashMap<String, Vec<u32>>` for $O(1)$ random encounter generation.
3. **Database Indexing**: Case-insensitive unique indexing (`idx_players_lower_username`) and descending XP indexing (`idx_players_xp_desc`) provide $O(\log N)$ user lookups and instantaneous leaderboard queries.
4. **Procedural Web Audio**: Sound effects use synthesized oscillators (`AudioContext`), minimizing network asset requests.
5. **Optimized SVG Pixel Sprite Sheet**: All 37 retro icons are pre-compiled into a single SVG symbol container (`<symbol id="px-...">`), reducing DOM overhead and eliminating external image round-trips.
