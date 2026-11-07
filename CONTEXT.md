# CONTEXT.md — PokéDash Advance Deep Context

This file documents the architectural decisions, mechanics, data flows, database schemas, and API contracts of the **PokéDash Advance** engine.

---

## 1. System Overview & Architecture

PokéDash Advance is designed as a standalone, zero-dependency client-server application packaged as a single self-contained binary.

```
┌─────────────────────────────────────────────────────────────┐
│                       Browser Client                        │
│   (static/index.html — GBA Handheld Console LCD Theme)      │
│  - Web Audio Procedural SFX  - Custom PixelEmulator Font    │
│  - Retro SVG Pixel Icon Set  - Keyboard & D-Pad Bindings    │
└──────────────────────────────┬──────────────────────────────┘
                               │ HTTP / JSON REST API
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                    Rust Axum Web Server                     │
│                (src/server.rs & src/service.rs)             │
│  - Routes: /api/hunt, /api/catch, /api/battle, /api/arena   │
│  - In-Memory RWLock State: Player, Encounter, Battle        │
│  - Static Asset Delivery: /sprites, /fonts, index.html      │
└──────────────┬──────────────────────────────┬───────────────┘
               │                              │
               ▼                              ▼
┌──────────────────────────────┐ ┌────────────────────────────┐
│    Cloud Database Layer      │ │   Local Storage Fallback   │
│       (src/supabase.rs)      │ │      (src/storage.rs)      │
│ - PostgREST HTTPS Client     │ │ - Path: ~/.pokedash/       │
│ - Tables: players, pokedex,  │ │ - Files: player.json,      │
│   wild_encounters, arena     │ │   pokedex.json, etc.       │
└──────────────────────────────┘ └────────────────────────────┘
```

---

## 2. Core Gameplay Loops

### 2.1 Wild Hunting & Encounters (`/api/hunt`)
- Triggered by `[SPACE]` or clicking **SEARCH TALL GRASS**.
- Awards **+5 XP** per hunt attempt.
- Pokémon selection is determined by a weighted random roll across 7 rarity tiers:
  - **Common (50%)**, **Uncommon (28%)**, **Rare (12%)**, **UltraRare (6%)**, **Epic (2.5%)**, **Legendary (1%)**, **Mythical (0.5%)**.
- Generates wild level based on the trainer level with variance: $\text{Level} \in [\max(1, L-3), L+2]$.
- Wild encounter state is cached in server memory and persisted to the database.

### 2.2 Catching Mechanics (`/api/catch`)
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
  - Added to Pokédex with timestamp and catch count.
  - Added to active battle party (up to 6 slots).
  - Rewards base catch XP (e.g. Common +10 XP, Legendary +500 XP).
  - If it is a **New Pokédex entry**, catch XP is **doubled**.

### 2.3 Wild Turn-Based Battle (`/api/battle/wild-attack`)
- The player's active lead Pokémon and wild Pokémon battle using real Type Effectiveness:
  - Weakness ($2.0\times$, Super Effective)
  - Resistance ($0.5\times$, Not Very Effective)
  - Neutral ($1.0\times$)
- Attacking lowers enemy HP. If enemy HP reaches 0:
  - Victory awarded with scaling currency (`P$`) and XP.
  - Active wild encounter is cleared.

### 2.4 Gym & Champion League Arena (`/api/arena`, `/api/arena/start`, `/api/arena/attack`)
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

### 2.5 Pokémon Center & Nurse Joy (`/api/battle/heal`)
- Restores all party Pokémon to full HP.
- Displays animated 24x24 retro Nurse Joy pixel portrait (`#px-nurse`).

### 2.6 Poké Mart Store (`/api/store`, `/api/store/buy`, `/api/store/sell-dupes`)
- Buy specialized Poké Balls.
- Level-based store discount: up to $15\%$ off based on trainer level.
- One-click bulk sale for duplicate Pokémon.

---

## 3. Database Schema (Supabase / PostgreSQL)

When connected to Supabase, the following tables in public schema are used:

```sql
-- 1. Player Data Table
CREATE TABLE IF NOT EXISTS public.players (
    username TEXT PRIMARY KEY,
    currency BIGINT NOT NULL DEFAULT 500,
    experience BIGINT NOT NULL DEFAULT 0,
    inventory JSONB NOT NULL DEFAULT '{}'::jsonb,
    party JSONB NOT NULL DEFAULT '[]'::jsonb,
    stats JSONB NOT NULL DEFAULT '{}'::jsonb,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 2. Caught Pokémon Catalog (Pokédex)
CREATE TABLE IF NOT EXISTS public.pokedex (
    id SERIAL PRIMARY KEY,
    username TEXT NOT NULL,
    pokemon_id INT NOT NULL,
    pokemon_data JSONB NOT NULL,
    first_caught_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    times_caught INT NOT NULL DEFAULT 1,
    UNIQUE (username, pokemon_id)
);

-- 3. Active Wild Encounters
CREATE TABLE IF NOT EXISTS public.wild_encounters (
    username TEXT PRIMARY KEY,
    encounter_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. Active Arena Battles
CREATE TABLE IF NOT EXISTS public.arena_battles (
    username TEXT PRIMARY KEY,
    battle_data JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## 4. REST API Specification

| Endpoint | Method | Description |
|---|---|---|
| `/api/status` | `GET` | Health check & trainer level status |
| `/api/player` | `GET` | Returns active player, level, party & stats |
| `/api/player/reset` | `POST` | Resets game state to starting defaults |
| `/api/hunt` | `POST` | Searches tall grass, rolls encounter (+5 XP) |
| `/api/wild` | `GET` | Returns currently active wild encounter |
| `/api/catch` | `POST` | Throws selected ball (`{ "ball": "pb" }`) |
| `/api/flee` | `POST` | Flees from current wild Pokémon |
| `/api/battle/wild-attack` | `POST` | Executes move against wild Pokémon |
| `/api/battle/heal` | `POST` | Nurse Joy restores party team HP |
| `/api/party/select` | `POST` | Sets lead party Pokémon |
| `/api/arena` | `GET` | Lists all 10 gym divisions & badges owned |
| `/api/arena/start` | `POST` | Challenges a gym leader (`{ "arena_id": "pewter" }`) |
| `/api/arena/attack` | `POST` | Executes move against leader Pokémon |
| `/api/pokedex` | `GET` | Filtered list of owned/caught Pokémon |
| `/api/catalog` | `GET` | Full 1025 National Pokédex catalog with ownership |
| `/api/pokemon/:id` | `GET` | Deep detail for specific Pokémon species |
| `/api/store` | `GET` | Store catalog, ball prices, discounts |
| `/api/store/buy` | `POST` | Purchases items (`{ "item": "gb", "amount": 5 }`) |
| `/api/store/sell` | `POST` | Sells single Pokémon species |
| `/api/store/sell-all` | `POST` | Sells all Pokémon of a specific rarity |
| `/api/store/sell-dupes` | `POST` | Sells all duplicate caught Pokémon |
