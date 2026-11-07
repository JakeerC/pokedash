# PokéDash ⚡

A retro Pokémon hunting, catching, battling, and collecting game powered by a high-performance **Rust backend engine** (Axum, Tokio, Serde), a custom **Game Boy Advance LCD Web UI**, and **Supabase PostgreSQL cloud persistence & GoTrue email authentication** with PokeAPI integration.

Inspired by [PokéCatch](https://pokecatch.ritikkumar.dev/) and powered by [PokeAPI](https://pokeapi.co/docs/v2).

---

## 🌟 Key Features

- **Email-Based Trainer Passport Authentication**:
  - **Email as Primary Identity**: Secure authentication powered by Supabase GoTrue Auth (`/auth/v1`).
  - **Unique Trainer Names (`playerName`)**: Globally unique, case-insensitive trainer handles enforced at both database (`lower(username)`) and application levels.
  - **Friction-Free Verification**: Supports 6-digit confirmation codes as well as 1-click email confirmation links (`#access_token=...`) that automatically log you in.
  - **Strict Gameplay Gating**: Unauthenticated gameplay is completely restricted. All game actions require valid Bearer JWT session tokens, and the frontend locks controls until signed in.

- **Real-Time Multiplayer Leaderboard**:
  - **Global Ranks**: Competitively ranks all trainers globally based on total experience points (`xp DESC`).
  - **Live Lead Pokémon**: Displays each trainer's active party lead Pokémon name, level, and animated sprite.
  - **Stats Tracking**: Displays player level, badges acquired (out of 10), and unique Pokémon caught (out of 1025).

- **Dedicated Retro Web UI (`static/index.html`)**:
  - **Authentic GBA LCD Aesthetics**: Custom dark emerald matrix LCD styling, optional CRT scanlines toggle, physical D-Pad, A/B action buttons, and top console slide switches.
  - **Embedded Pixel Fonts**: High-legibility retro typography utilizing embedded `PixelEmulator` font.
  - **Zero-Emoji Pixel Art**: 37 custom-drawn 8-bit/16-bit SVG pixel art icons (grass, swords, bag, pokeball, boots, gold coin, hearts, speaker, and full gym badges) plus a 24x24 animated Nurse Joy portrait.
  - **Wild Encounter Arena**: Animated Showdown Pokémon battle sprites, health bars, attack lunges, damage shakes, and capture animations.
  - **Official Audio & Procedural SFX**: Web Audio API retro chiptune sound effects combined with official Pokémon audio cries streamed dynamically from PokeAPI.
  - **Turn-Based Battles**: Type effectiveness system (Super Effective $2.0\times$, Not Very Effective $0.5\times$, Neutral $1.0\times$) with party lead swapping and move selection.
  - **Pokédex & Catalog**: Responsive 4-column national Pokédex card grid with enlarged 120px sprites, generation filters (Gen I–IX), dual-type filters, rarity filters, and deep 72% screen stat breakdown modals.
  - **Gym League & Arena**: 10 Gym Divisions (Brock through Champion Cynthia) with team previews, dialogue intros, and collectible custom pixel badges.
  - **Poké Mart Store**: 11 specialized Poké Balls, level-scaling store discounts (up to 15%), and 1-click duplicate Pokémon sales.
  - **Dual Input Controls**: Complete parity between mouse clicks and tactile keyboard controls (`[SPACE]`, `[1-4]`, `[W/A/S/D]`, `[H]`, `[C]`, `[Z/ENTER]`, `[X/ESC]`).

- **High-Performance Rust Backend**:
  - Built with **Axum 0.7**, **Tokio**, and **Tower-HTTP**.
  - All 1025 Pokémon definitions pre-loaded in memory for $O(1)$ weighted encounter lookups.
  - High-speed REST API endpoints with in-memory JWT authentication cache (10-minute TTL).

- **Supabase Cloud Persistence & Fail-Safe Offline Mode**:
  - Direct integration with **Supabase PostgREST** for cross-device cloud persistence.
  - Multi-tenant storage scoped by trainer username (`player_<username>.json`, `pokedex_<username>.json`).
  - Transparent fallback to local JSON in `~/.pokedash/` if offline or unconfigured.

- **Agentic AI Ready**:
  - Comprehensive documentation, skills, and architectures designed for AI developer agents (`AGENTS.md`, `CONTEXT.md`, `ARCHITECTURE.md`, `.agents/`, `.gemini/`).

---

## 🚀 Quick Start

### 1. Build

```bash
cargo build --release
```

The compiled standalone binary is placed at `./target/release/pokedash`.

### 2. Configure Cloud Database & Authentication (Optional)

Copy `.env.example` to `.env`:

```bash
cp .env.example .env
```

Set your Supabase credentials:
```bash
SUPABASE_URL=https://<your-project-ref>.supabase.co
SUPABASE_KEY=<your-anon-or-service-key>
```
*Note: If no Supabase environment variables are provided, the game falls back to local JSON storage in `~/.pokedash/`.*

### 3. Launch the Web Game

```bash
./target/release/pokedash --port 8080 --open
```

Open your browser to: **`http://localhost:8080`**

On initial launch, the **Silph Co. Trainer Terminal** will appear:
1. Click **REGISTER** to create your unique Trainer Passport (email, password, and chosen trainer name).
2. Enter your 6-digit confirmation code (or click the email confirmation link).
3. Begin hunting, battling, and collecting Pokémon!

### Command-line Options:
```
Options:
  -p, --port <PORT>  Port to listen on [default: 8080]
      --host <HOST>  Host interface to bind to [default: 0.0.0.0]
      --open         Automatically open in default browser
  -h, --help         Print help
```

---

## 🎮 Keyboard Controls

| Key | Action |
|---|---|
| `[SPACE]` | Search Tall Grass for Wild Pokémon / Confirm Dialog |
| `[1]` | Select **FIGHT** Menu in Battle |
| `[2]` | Open **BAG** Menu to select Poké Balls |
| `[3]` | Open **PKMN** Party Menu |
| `[4]` | **RUN** / Flee from Wild Pokémon |
| `[W]` / `[↑]` | Scroll Screen Up |
| `[S]` / `[↓]` | Scroll Screen Down |
| `[A]` / `[←]` | Navigate to Previous Tab |
| `[D]` / `[→]` | Navigate to Next Tab |
| `[H]` | Nurse Joy: Restore Team HP (Poké Center) |
| `[C]` | Play Active Pokémon Audio Cry |
| `[Z]` / `[ENTER]` | Physical **A** Action Button |
| `[X]` / `[ESC]` | Physical **B** Cancel Button / Close Modals |

---

## 🎯 Mechanics & Formulas

### Encounter Odds

| Rarity | Spawn Weight | Base Catch Rate | Sell Value |
|---|---|---|---|
| Common | 50.0% | 50% | ₱150 |
| Uncommon | 28.0% | 40% | ₱300 |
| Rare | 12.0% | 30% | ₱800 |
| UltraRare | 6.0% | 20% | ₱1,500 |
| Epic | 2.5% | 10% | ₱3,000 |
| Legendary | 1.0% | 5% | ₱10,000 |
| Mythical | 0.5% | 2% | ₱25,000 |

### Catch Rate Formula

$$\text{Catch Chance} = \min\Big(\text{Base Rate} \times \text{Ball Multiplier} \times (1 + \text{Level Bonus}), 1.0\Big)$$

- **Level Bonus**: $+1\%$ bonus per level, capped at $+25\%$ at Level 25.
- **Ball Multipliers**:
  - `pb` (Poké Ball): $1.0\times$
  - `gb` (Great Ball): $1.5\times$
  - `ub` (Ultra Ball): $2.0\times$
  - `net` (Net Ball): $3.0\times$ on Water or Bug types
  - `dive` (Dive Ball): $3.5\times$ on Water types
  - `fast` (Fast Ball): $2.0\times$ if Speed $\ge 80$, $3.0\times$ if Speed $\ge 120$
  - `dusk` (Dusk Ball): $3.5\times$ at night (18:00 to 06:00 local time)
  - `nest` (Nest Ball): $2.0\times$ if BST $\le 400$, $3.0\times$ if BST $\le 300$
  - `repeat` (Repeat Ball): $3.0\times$ if species is already in your Pokédex
  - `quick` (Quick Ball): $2.5\times$ flat boost
  - `mb` (Master Ball): Guaranteed $100\%$ catch

### XP and Progression

$$\text{XP Required for Level } L = \lfloor 100 \times L^{1.5} \rfloor$$

- **Hunt**: $+5\text{ XP}$
- **Catch**: Common $+10$, Uncommon $+20$, Rare $+50$, UltraRare $+100$, Epic $+200$, Legendary $+500$, Mythical $+1000\text{ XP}$.
- **New Species Bonus**: Catch XP is **doubled** for catching an unowned species!
- **Store Discount**: $0.5\%$ discount per level up to $15\%$ maximum discount.

---

## 🗄️ Database & Cloud Persistence (Supabase)

PokéDash uses **Supabase PostgreSQL** as its primary cloud database, providing durable, multi-tenant persistence and global leaderboard synchronization across devices.

### Tables & Key Indexes

| Table | Description | Key Constraint / Index |
|---|---|---|
| `players` | Stores player profile, level, currency, XP, Pokéball inventory, party Pokémon, and badges. | `PRIMARY KEY (username)`, `UNIQUE (lower(username))`, `INDEX (xp DESC)` |
| `pokedex` | Stores every caught Pokémon instance with stats, IVs, rarity, catch timestamps, and ball used. | `PRIMARY KEY (entry_id)`, `INDEX (username)` |
| `wild_encounters` | Persists active wild Pokémon encounter state, remaining HP, and cooldowns. | `PRIMARY KEY (username)` |
| `arena_battles` | Persists ongoing gym arena battles, active Pokémon, and combat logs. | `PRIMARY KEY (username)` |

---

## 📡 REST API Overview

### Public Routes
- `GET /api/status` — Server health, engine version, and auth configuration.
- `GET /api/leaderboard` — Real-time top 50 global multiplayer rankings.

### Authentication Routes
- `POST /api/auth/signup` — Register with email, password, and unique trainer name.
- `POST /api/auth/login` — Sign in with email and password to receive Bearer JWT.
- `POST /api/auth/verify` — Confirm email code to activate account.
- `POST /api/auth/resend` — Resend verification code.
- `GET /api/auth/me` — Verify current session token and retrieve trainer identity.

### Protected Gameplay Routes (Requires `Authorization: Bearer <token>`)
- `GET /api/player` & `POST /api/player/reset` — Player profile and reset.
- `POST /api/hunt`, `GET /api/wild`, `POST /api/catch`, `POST /api/flee` — Tall grass hunting and catching.
- `POST /api/battle/wild-attack` & `POST /api/battle/heal` — Turn-based combat and nurse healing.
- `POST /api/party/select` — Active battle team management.
- `GET /api/arena`, `POST /api/arena/start`, `POST /api/arena/attack` — Gym leader divisions and battles.
- `GET /api/pokedex`, `GET /api/catalog`, `GET /api/pokemon/:id` — Pokédex entries and species details.
- `GET /api/store`, `POST /api/store/buy`, `POST /api/store/sell`, `POST /api/store/sell-dupes` — Mart purchasing and selling.

---

## 🤖 Agentic AI & Developer Documentation

PokéDash includes dedicated documentation and skill definitions for autonomous coding agents:

- **[`AGENTS.md`](AGENTS.md)**: Rules of engagement, coding directives, zero-emoji policy, and definition of done for AI agents.
- **[`CONTEXT.md`](CONTEXT.md)**: Exhaustive gameplay loops, mathematical formulas, Supabase SQL schemas, and REST API specification.
- **[`ARCHITECTURE.md`](ARCHITECTURE.md)**: Deep architectural guide, security model, concurrency model (`Arc<RwLock<()>>`), write-through storage flow, and latency optimizations.
- **[`.agents/skills/`](.agents/skills/)** & **[`.gemini/skills/`](.gemini/skills/)**: Skill definitions for agent task execution and validation.

---

## 🛠️ Project Structure

```
pokedash/
├── Cargo.toml            # Rust dependencies & build settings
├── .env.example          # Supabase environment variables template
├── AGENTS.md             # Directives and guidelines for AI agents
├── CONTEXT.md            # Deep gameplay mechanics, API & database schema
├── ARCHITECTURE.md       # Architecture, security, concurrency, and persistence design
├── .agents/skills/       # Agent skills for PokéDash core standards
├── .gemini/skills/       # Gemini agent skills
├── data/
│   └── pokemon_data.json # All 1025 Pokemon stats, types, rarities
├── static/
│   ├── index.html        # Retro GBA Web Game UI, Auth Modal & Dashboard
│   ├── pixel-icons.svg   # 37 custom-drawn retro pixel art SVG icons
│   ├── keyboard.html     # On-screen keyboard input test interface
│   ├── fonts/            # PixelEmulator and Player2 retro font assets
│   └── sprites/          # Bundled sprites (1.png - 1025.png)
├── tools/
│   ├── generate_pixel_icons.py # Pixel icon SVG symbol compiler
│   └── apply_pixel_icons.py    # Pixel icon injector script
├── src/
│   ├── main.rs           # Application entry point & initialization
│   ├── lib.rs            # Library module exports
│   ├── models.rs         # Strongly typed data models (Player, Auth, Leaderboard)
│   ├── game.rs           # Core mechanics, odds, ball formulas, gym divisions
│   ├── supabase.rs       # Supabase GoTrue client, token cache, and PostgREST ops
│   ├── storage.rs        # Dual persistence: Supabase DB + local fallback
│   ├── service.rs        # Core gameplay service & lock management (user scoped)
│   ├── server.rs         # Axum HTTP API, AuthenticatedTrainer extractor & routing
│   └── cli.rs            # Web server arguments & terminal banner
└── README.md             # Project documentation
```
