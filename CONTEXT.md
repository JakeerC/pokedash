# CONTEXT.md — PokéDash Advance Deep Context

This file documents the architectural decisions, mechanics, data flows, database schemas, security contracts, and API specifications of the **PokéDash Advance** engine.

---

## 1. System Overview & Architecture

PokéDash Advance is designed as a standalone, zero-dependency client-server application packaged as a single self-contained binary, integrated with **Supabase PostgreSQL & GoTrue Auth** for authentication, multi-tenant persistence, and global leaderboards.

```
┌─────────────────────────────────────────────────────────────┐
│                       Browser Client                        │
│   (static/index.html — GBA Handheld Console LCD Theme)      │
│  - Retro Auth Modal (Sign In, Register, Confirm Code)       │
│  - URL Hash Token Extractor (#access_token=...)             │
│  - Web Audio Procedural SFX  - Custom PixelEmulator Font    │
│  - Retro SVG Pixel Icon Set  - Keyboard & D-Pad Bindings    │
│  - Input Lockout Guard (Blocks unauthenticated controls)    │
└──────────────────────────────┬──────────────────────────────┘
                               │ HTTPS / JSON REST API (Bearer JWT)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    Rust Axum Web Server                     │
│                (src/server.rs & src/service.rs)             │
│  - Auth Extractor: AuthenticatedTrainer (10-min TTL cache)  │
│  - Auth Routes: /api/auth/signup, /login, /verify, /resend  │
│  - Public Discovery: /api/status, /api/leaderboard          │
│  - Protected Game Routes: /api/player, /api/hunt, /catch... │
│  - In-Memory RWLock State: Player, Encounter, Battle        │
│  - Static Asset Delivery: /sprites, /fonts, index.html      │
└──────────────┬──────────────────────────────┬───────────────┘
               │                              │
               ▼                              ▼
┌──────────────────────────────┐ ┌────────────────────────────┐
│    Cloud Database Layer      │ │   Local Storage Fallback   │
│       (src/supabase.rs)      │ │      (src/storage.rs)      │
│ - Supabase Auth (GoTrue API) │ │ - Path: ~/.pokedash/       │
│ - PostgREST HTTPS Client     │ │ - Files:                   │
│ - Tables: players, pokedex,  │ │   player_<username>.json,  │
│   wild_encounters, arena     │ │   pokedex_<username>.json  │
│ - Indexes: lower(username),  │ │ - Automatic sync on boot   │
│   xp DESC (leaderboard)      │ │                            │
└──────────────────────────────┘ └────────────────────────────┘
```

---

## 2. Authentication & Identity Specification

### 2.1 Default Email Authentication
- **Provider**: Supabase GoTrue Auth API (`/auth/v1`).
- **Registration Flow** (`POST /api/auth/signup`):
  - Request body: `{ "email": "...", "password": "...", "username": "..." }`.
  - Trainer username is verified: 3–20 characters, alphanumeric (`_` and `-` permitted).
  - Availability is pre-checked case-insensitively against `public.players`.
  - Supabase Auth creates the identity with `raw_user_meta_data: { "username": "<playerName>" }`.
  - Default player profile is initialized in `public.players`.
  - If email verification is enabled, returns `{ "requires_verification": true }`.
- **Sign In Flow** (`POST /api/auth/login`):
  - Request body: `{ "email": "...", "password": "..." }`.
  - Returns `{ "token": "<jwt>", "user": { "id": "...", "email": "...", "username": "..." } }`.
  - The JWT access token is stored in the client's `localStorage` as `pokedash_session_token`.
- **Email Confirmation Flow** (`POST /api/auth/verify`):
  - Request body: `{ "email": "...", "token": "<6-digit-code>", "type": "signup" }`.
  - Verifies email token, returns new session JWT, and activates the Trainer ID.
- **Resend Code** (`POST /api/auth/resend`):
  - Request body: `{ "email": "..." }`.
  - Dispatches a fresh confirmation code via Supabase.
- **Email Link Redirect Support**:
  - Clicking the email verification link in confirmation emails redirects to the web game URL with an access token in the hash fragment: `http://localhost:8080/#access_token=...&type=signup`.
  - On `DOMContentLoaded`, the frontend detects and extracts the token, queries `GET /api/auth/me`, removes the hash from the browser URL bar, and seamlessly authenticates the trainer without any manual code entry.

### 2.2 Unique `playerName`
- Every trainer must have a unique `playerName` (`username`).
- **Case-Insensitive Constraint**: Database index `idx_players_lower_username` on `public.players (lower(username))` guarantees that names like `Ash`, `ash`, and `ASH` cannot be duplicated.
- **Duplicate Prevention**: `SupabaseClient::is_username_taken` checks `username=ilike.<username>`. Duplicates are rejected with `HTTP 409 Conflict`.

### 2.3 Strict Access Control (Playing Restricted Without Auth)
- **Backend Protection**: Every gameplay route extracts [`AuthenticatedTrainer`](src/server.rs). If the `Authorization: Bearer <token>` header is missing, malformed, or invalid, the server responds with `HTTP 401 Unauthorized` (`{"code": "unauthorized"}`).
- **In-Memory Token Cache**: Validated tokens are cached with a 10-minute TTL in an `RwLock<HashMap<String, (AuthUser, Instant)>>` inside `SupabaseClient` to eliminate Supabase Auth API latency on gameplay actions.
- **Frontend Interception**: When unauthenticated, `#auth-modal` is displayed as an inescapable modal overlay. All keyboard shortcuts and GBA D-Pad / action buttons are blocked.

---

## 3. Multiplayer & Global Leaderboards

- **Global Leaderboard Endpoint** (`GET /api/leaderboard`):
  - Publicly accessible endpoint returning the top 50 trainers ranked by experience points (`xp DESC`).
  - Indexed by `idx_players_xp_desc` for $O(\log N)$ performance.
- **Leaderboard Payload**:
  - `rank`: Leaderboard placement (#1, #2, ...).
  - `username`: Unique trainer handle.
  - `level`: Computed from XP ($\lfloor 100 \times L^{1.5} \rfloor$).
  - `xp`: Total lifetime experience points.
  - `currency`: Current Pokédollars (`P$`).
  - `badges_count`: Number of gym badges acquired (out of 10).
  - `unique_caught`: Number of distinct Pokémon species registered in the Pokédex (out of 1025).
  - `lead_pokemon_id`, `lead_pokemon_name`, `lead_pokemon_level`, `lead_pokemon_sprite`: Live lead party Pokémon data.

---

## 4. Core Gameplay Loops

### 4.1 Wild Hunting & Encounters (`/api/hunt`)
- Triggered by `[SPACE]` or clicking **SEARCH TALL GRASS**.
- Awards **+5 XP** per hunt attempt.
- Pokémon selection is determined by a weighted random roll across 7 rarity tiers:
  - **Common (50%)**, **Uncommon (28%)**, **Rare (12%)**, **UltraRare (6%)**, **Epic (2.5%)**, **Legendary (1%)**, **Mythical (0.5%)**.
- Generates wild level based on the trainer level with variance: $\text{Level} \in [\max(1, L-3), L+2]$.
- Wild encounter state is cached per trainer and persisted to Supabase `wild_encounters`.

### 4.2 Catching Mechanics (`/api/catch`)
- Supported Poké Balls (11 types):
  - `pb` (Poké Ball): $1.0\times$
  - `gb` (Great Ball): $1.5\times$
  - `ub` (Ultra Ball): $2.0\times$
  - `net` (Net Ball): $3.0\times$ if Pokémon type contains Water or Bug
  - `dive` (Dive Ball): $3.5\times$ if Pokémon type contains Water
  - `fast` (Fast Ball): $2.0\times$ if Speed $\ge 80$, $3.0\times$ if Speed $\ge 120$
  - `dusk` (Dusk Ball): $3.5\times$ at night (18:00 to 06:00 local time)
  - `nest` (Nest Ball): $2.0\times$ if BST $\le 400$, $3.0\times$ if BST $\le 300$
  - `repeat` (Repeat Ball): $3.0\times$ if already caught previously
  - `quick` (Quick Ball): $2.5\times$ base boost
  - `mb` (Master Ball): Guaranteed $100\%$ catch
- **Catch Rate Formula**:
  $$\text{Chance} = \min\Big(\text{Base Rate} \times \text{Multiplier} \times (1 + \min(0.25, 0.01 \times \text{Trainer Level})), 1.0\Big)$$
- If caught:
  - Added to trainer's Pokédex with timestamp and catch count.
  - Added to active battle party (up to 6 slots).
  - Rewards base catch XP (e.g. Common +10 XP, Legendary +500 XP).
  - If it is a **New Pokédex entry**, catch XP is **doubled**.

### 4.3 Wild Turn-Based Battle (`/api/battle/wild-attack`)
- The player's active lead Pokémon and wild Pokémon battle using real Type Effectiveness:
  - Super Effective ($2.0\times$)
  - Not Very Effective ($0.5\times$)
  - Neutral ($1.0\times$)
- Attacking lowers enemy HP. If enemy HP reaches 0:
  - Victory awarded with scaling currency (`P$`) and XP.
  - Active wild encounter is cleared.

### 4.4 Gym & Champion League Arena (`/api/arena`, `/api/arena/start`, `/api/arena/attack`)
- 10 structured Gym Divisions:
  1. Pewter Gym (Brock) — Boulder Badge (`boulder`)
  2. Cerulean Gym (Misty) — Cascade Badge (`cascade`)
  3. Vermilion Gym (Lt. Surge) — Thunder Badge (`thunder`)
  4. Celadon Gym (Erika) — Rainbow Badge (`rainbow`)
  5. Saffron Gym (Sabrina) — Marsh Badge (`marsh`)
  6. Fuchsia Gym (Koga) — Soul Badge (`soul`)
  7. Cinnabar Gym (Blaine) — Volcano Badge (`volcano`)
  8. Viridian Gym (Giovanni) — Earth Badge (`earth`)
  9. Indigo Plateau (Lance) — Dragon Crest (`dragon`)
  10. Master Battle Tower (Cynthia) — Master Trophy (`champion`)
- Completing a division grants cash prize, XP, and unlocks the respective custom pixel badge in the Trainer Passport.

### 4.5 Pokémon Center & Nurse Joy (`/api/battle/heal`)
- Restores all party Pokémon to full HP.
- Displays animated 24x24 retro Nurse Joy pixel portrait (`#px-nurse`).

### 4.6 Poké Mart Store (`/api/store`, `/api/store/buy`, `/api/store/sell-dupes`)
- Buy specialized Poké Balls.
- Level-based store discount: up to $15\%$ off based on trainer level.
- One-click bulk sale for duplicate Pokémon.

---

## 5. Database Schema (Supabase / PostgreSQL)

When connected to Supabase, the following tables and indexes in the `public` schema are active:

```sql
-- 1. Player Data Table
CREATE TABLE IF NOT EXISTS public.players (
    username TEXT PRIMARY KEY,
    currency BIGINT NOT NULL DEFAULT 500,
    xp BIGINT NOT NULL DEFAULT 0,
    balls JSONB NOT NULL DEFAULT '{"poke_ball": 50}'::jsonb,
    last_hunt_time DOUBLE PRECISION NOT NULL DEFAULT 0,
    stats JSONB NOT NULL DEFAULT '{}'::jsonb,
    active_pokemon_id INT,
    party JSONB NOT NULL DEFAULT '[]'::jsonb,
    arena_badges JSONB NOT NULL DEFAULT '[]'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes on Players Table
CREATE UNIQUE INDEX IF NOT EXISTS idx_players_lower_username ON public.players (lower(username));
CREATE INDEX IF NOT EXISTS idx_players_xp_desc ON public.players (xp DESC);

-- 2. Caught Pokémon Catalog (Pokédex)
CREATE TABLE IF NOT EXISTS public.pokedex (
    entry_id BIGSERIAL PRIMARY KEY,
    username TEXT NOT NULL,
    id INT NOT NULL,
    name TEXT NOT NULL,
    types JSONB NOT NULL,
    abilities JSONB NOT NULL,
    stats JSONB NOT NULL,
    rarity TEXT NOT NULL,
    rarity_score DOUBLE PRECISION NOT NULL DEFAULT 0,
    caught_at DOUBLE PRECISION NOT NULL,
    ball_used TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pokedex_username ON public.pokedex (username);
CREATE INDEX IF NOT EXISTS idx_pokedex_caught_at ON public.pokedex (caught_at);

-- 3. Active Wild Encounters
CREATE TABLE IF NOT EXISTS public.wild_encounters (
    username TEXT PRIMARY KEY,
    encounter JSONB,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. Active Arena Battles
CREATE TABLE IF NOT EXISTS public.arena_battles (
    username TEXT PRIMARY KEY,
    battle JSONB,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 5. Atomic RPC Pokédex Batch Save
CREATE OR REPLACE FUNCTION public.save_pokedex(p_username text, p_entries jsonb)
RETURNS void AS $$
BEGIN
    DELETE FROM public.pokedex WHERE username = p_username;
    IF jsonb_array_length(p_entries) > 0 THEN
        INSERT INTO public.pokedex (username, id, name, types, abilities, stats, rarity, rarity_score, caught_at, ball_used)
        SELECT
            p_username,
            (elem->>'id')::int,
            elem->>'name',
            elem->'types',
            elem->'abilities',
            elem->'stats',
            elem->>'rarity',
            (elem->>'rarity_score')::double precision,
            (elem->>'caught_at')::double precision,
            elem->>'ball_used'
        FROM jsonb_array_elements(p_entries) AS elem;
    END IF;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;
```

---

## 6. REST API Specification

### 6.1 Public Endpoints (No Authentication Required)

| Endpoint | Method | Description |
|---|---|---|
| `/api/status` | `GET` | Health check, server version, and auth configuration |
| `/api/leaderboard` | `GET` | Top 50 global multiplayer trainers ranked by XP |
| `/keyboard` | `GET` | Standalone on-screen keyboard testing interface |

### 6.2 Authentication Endpoints

| Endpoint | Method | Payload / Response | Description |
|---|---|---|---|
| `/api/auth/signup` | `POST` | `{ "email", "password", "username" }` | Registers a new trainer with unique `username` |
| `/api/auth/login` | `POST` | `{ "email", "password" }` | Authenticates and returns Bearer JWT session |
| `/api/auth/verify` | `POST` | `{ "email", "token", "type" }` | Verifies 6-digit confirmation code |
| `/api/auth/resend` | `POST` | `{ "email" }` | Resends confirmation email/code |
| `/api/auth/me` | `GET` | Bearer Token $\rightarrow$ `{ "user": { ... } }` | Validates session token and returns identity |

### 6.3 Protected Gameplay Endpoints (Require `Authorization: Bearer <token>`)

| Endpoint | Method | Description |
|---|---|---|
| `/api/player` | `GET` | Returns trainer profile, level, party, and stats |
| `/api/player/reset` | `POST` | Resets trainer game state to starting defaults |
| `/api/hunt` | `POST` | Searches tall grass for a wild Pokémon encounter (+5 XP) |
| `/api/wild` | `GET` | Returns currently active wild encounter |
| `/api/catch` | `POST` | Throws selected ball (`{ "ball": "poke_ball" }`) |
| `/api/flee` | `POST` | Flees from current wild encounter |
| `/api/battle/wild-attack` | `POST` | Executes selected move against wild Pokémon |
| `/api/battle/heal` | `POST` | Restores all party Pokémon HP via Nurse Joy |
| `/api/party/select` | `POST` | Selects lead Pokémon (`{ "pokemon_id": 25 }`) |
| `/api/arena` | `GET` | Lists 10 gym divisions and unlocked badges |
| `/api/arena/start` | `POST` | Challenges a gym leader (`{ "arena_id": "pewter" }`) |
| `/api/arena/attack` | `POST` | Executes selected move against gym leader Pokémon |
| `/api/pokedex` | `GET` | Filtered list of owned/caught Pokémon |
| `/api/catalog` | `GET` | Full 1025 National Pokédex catalog with ownership flags |
| `/api/pokemon/:id` | `GET` | Detailed stats, cries, artwork, and catch history for species |
| `/api/store` | `GET` | Store catalog, ball prices, level discounts |
| `/api/store/buy` | `POST` | Purchases items (`{ "item": "poke_ball", "amount": 5 }`) |
| `/api/store/sell` | `POST` | Sells single caught Pokémon |
| `/api/store/sell-all` | `POST` | Sells all caught Pokémon of a specific rarity tier |
| `/api/store/sell-dupes` | `POST` | Bulk sells all duplicate caught Pokémon |
