# PokéDash ⚡

A Pokémon hunting, catching, and collecting web game powered by a high-performance **Rust backend engine** and modern retro **Game Boy / Synthwave Web UI** with official PokeAPI integration.

Inspired by [PokéCatch](https://pokecatch.ritikkumar.dev/) and powered by [PokeAPI](https://pokeapi.co/docs/v2).

---

## 🌟 Key Features

- **Dedicated Web Application**:
  - Interactive Wild Encounter arena with animated Showdown Pokémon sprites and high-res official artwork.
  - Official Pokémon audio cries streamed directly from PokeAPI.
  - Poké Ball selector with real-time calculated catch chance previews for every ball.
  - Throwing and catching animations with shake sequence and escape/capture outcomes.
  - Complete Pokédex with Generation tabs (Gen I–IX), Dual-type filtering, Rarity filtering, search, and detailed modal views.
  - Poké Mart Store with 11 specialized Poké Balls, Level-based discounts, and 1-click duplicate sales.
  - Trainer Passport with XP progress bar, leveling, currency, and achievement statistics.
- **High-Performance Rust Backend**:
  - Built with Axum, Tokio, and Serde.
  - All 1025 Pokémon embedded and loaded into memory on startup.
  - **Supabase PostgreSQL Cloud Persistence**: Direct integration with Supabase PostgREST API for robust cloud storage, with seamless auto-migration and local offline fallback in `~/.pokedash/`.
  - High-speed REST API endpoints for all gameplay operations.
- **Authentic Game Mechanics**:
  - 7 rarity tiers (Common to Mythical) with authentic spawn weights.
  - Situational multipliers for all 11 Poké Balls (Water/Bug bonuses, Speed check, Nighttime check, BST check, Repeat bonus, Master ball guarantee).
  - Progressive Trainer Level bonuses to base catch rates and store discounts.

---

## 🚀 Quick Start

### 1. Build

```bash
cargo build --release
```

The optimized binary is located at `./target/release/pokedash`.

### 2. Launch the Web Game

```bash
./target/release/pokedash --port 8080 --open
```

Open your browser to: **`http://localhost:8080`**

### Command-line Options:
```
Options:
  -p, --port <PORT>  Port to listen on [default: 8080]
      --host <HOST>  Host interface to bind to [default: 0.0.0.0]
      --open         Automatically open in default browser
  -h, --help         Print help
```

---

## 🎯 Mechanics & Formulas

### Encounter Odds

| Rarity | Spawn Weight | Base Catch Rate | Sell Value |
|---|---|---|---|
| Common | 50.0% | 50% | ₽150 |
| Uncommon | 28.0% | 40% | ₽300 |
| Rare | 12.0% | 30% | ₽800 |
| UltraRare | 6.0% | 20% | ₽1,500 |
| Epic | 2.5% | 10% | ₽3,000 |
| Legendary | 1.0% | 5% | ₽10,000 |
| Mythical | 0.5% | 2% | ₽25,000 |

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
  - `dusk` (Dusk Ball): $3.5\times$ at night (18:00 to 06:00)
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

PokéDash uses **Supabase PostgreSQL** as its primary cloud database, providing durable, scalable persistence across devices and sessions.

### Architecture & Tables

| Table | Description | Primary Key |
|---|---|---|
| `players` | Stores player profile, level, currency, XP, Pokéball inventory, party Pokémon, and badges. | `username` |
| `pokedex` | Stores every caught Pokémon instance with stats, IVs, rarity, catch timestamps, and ball used. | `entry_id` (indexed by `username`, `caught_at`) |
| `wild_encounters` | Persists active wild Pokémon encounter state, remaining HP, and cooldowns. | `username` |
| `arena_battles` | Persists ongoing gym arena battles, active Pokémon, and combat logs. | `username` |

- **Atomic Synchronization**: Catches, transfers, sales, and renames are safely synchronized via the `save_pokedex(p_username, p_entries)` database RPC.
- **Row Level Security (RLS)**: Enabled across all tables with explicit access policies for data security.
- **Auto-Migration & Offline Fallback**: On first launch with Supabase configured, existing local saves from `~/.pokedash/` are automatically migrated to Supabase. If offline, the game transparently falls back to local JSON storage.

### Configuration

Copy `.env.example` to `.env` or set the environment variables:

```bash
SUPABASE_URL=https://<your-project-ref>.supabase.co
SUPABASE_KEY=<your-anon-or-service-key>
```

---

## 🛠️ Project Structure

```
pokedash/
├── Cargo.toml            # Rust dependencies & config
├── .env.example          # Supabase environment variables template
├── data/
│   └── pokemon_data.json # All 1025 Pokemon stats, types, rarities
├── static/
│   ├── index.html        # Retro Game Boy/Synthwave Web Game & Dashboard
│   ├── keyboard.html     # On-screen keyboard input interface
│   └── sprites/          # Bundled sprites (1.png - 1025.png)
├── src/
│   ├── main.rs           # Application entry point & initialization
│   ├── lib.rs            # Library module exports
│   ├── models.rs         # Data models and structures
│   ├── game.rs           # Core mechanics, odds, ball formulas
│   ├── supabase.rs       # Supabase PostgREST client & environment loader
│   ├── storage.rs        # Dual persistence: Supabase DB + local fallback
│   ├── service.rs        # Core gameplay service & lock management
│   ├── server.rs         # Axum HTTP API & static web router
│   └── cli.rs            # Web server arguments & terminal banner
└── README.md             # Project documentation
```
