# PokéDash ⚡

A high-performance Pokémon hunting, catching, and collecting game playable on **both Terminal and Web**, powered by a **Rust backend engine** with seamless state synchronization and full PokeAPI integration.

Inspired by [PokéCatch](https://pokecatch.ritikkumar.dev/) and powered by [PokeAPI](https://pokeapi.co/docs/v2).

---

## 🌟 Key Features

- **Play Anywhere**:
  - **Terminal Mode**: Blazing-fast CLI in Rust (`pokedash`) and Python (`python3 pokedash_cli.py`) with rich ANSI colors, stat bars, and interactive menu navigation (`pokedash play`).
  - **Web Mode**: Retro Synthwave / Game Boy web application served directly by Rust at `http://localhost:8080`, featuring animated Showdown Pokémon sprites, official artwork, audio cries from PokeAPI, and real-time catch odds calculation.
  - **Shared State**: Play in your terminal, then open the browser — your catches, Poké Balls, level, and currency are instantly synchronized via `~/.pokedash`.
- **All 1025 Pokémon**:
  - Generations 1 through 9 (from Bulbasaur to Pecharunt).
  - 7 rarity tiers (Common to Mythical) with authentic encounter weightings.
  - Authentic stats (HP, Atk, Def, SpA, SpD, Spe, BST), types, and abilities.
- **11 Specialized Poké Balls**:
  - Each ball features situational bonuses (e.g. Net Ball for Water/Bug, Fast Ball for high Speed, Dusk Ball at night, Nest Ball for low BST, Repeat Ball for owned species).
  - Calculated catch chance preview in both terminal and web UI.
- **Dynamic Poké Mart & Economy**:
  - Earn Poké Dollars by selling duplicates (`sell-dupes`), selling by rarity (`sellall`), or single sales.
  - Level-based discounts at the Mart (up to 15% discount at higher levels).
- **Audio & Visual Immersion**:
  - High-res official artwork and animated showdown GIFs.
  - Authentic sound cries for all Pokémon streamed via PokeAPI cries CDN.

---

## 🚀 Quick Start

### 1. Build the Rust Engine & CLI

From the `pokedash` directory:

```bash
cargo build --release
```

The optimized binary will be at `./target/release/pokedash`.

### 2. Launch the Web Application

Start the embedded Axum HTTP server & Web UI:

```bash
./target/release/pokedash serve --port 8080 --open
```

Open your browser to: **`http://localhost:8080`**

---

## 💻 Terminal Commands

### Rust CLI Commands

| Command | Alias | Description |
|---|---|---|
| `pokedash hunt` | | Search the tall grass for a wild Pokémon (+5 XP) |
| `pokedash catch <ball>` | | Throw a Poké Ball (`pb`, `gb`, `ub`, `net`, `dive`, `fast`, `dusk`, `nest`, `repeat`, `quick`, `mb`) |
| `pokedash dex` | `pokedex` | View your caught Pokémon with pagination |
| `pokedash dex <name>` | | Inspect full stats and details for a specific Pokémon |
| `pokedash dex --gen 1 --type water` | | Filter Pokédex by Generation and Type |
| `pokedash dex --sort rarity` | | Sort Pokédex by rarity or BST |
| `pokedash inv` | `inventory` | Check your Poké Balls inventory and balance |
| `pokedash stats` | `s` | View Trainer Level, XP progress, and catch records |
| `pokedash store` | | Browse the Poké Mart catalog |
| `pokedash store buy <ball> [qty]`| | Buy Poké Balls with level discount applied |
| `pokedash store sell <name>` | | Sell a specific Pokémon |
| `pokedash store sellall <rarity>`| | Sell all Pokémon of a rarity tier |
| `pokedash store sell-dupes` | | Keep 1 of every species and sell all extra copies |
| `pokedash play` | | Launch interactive terminal arrow-key menu game loop |
| `pokedash serve` | | Run the web & REST API server |

### Python Companion CLI

If you prefer Python in your terminal:

```bash
python3 pokedash_cli.py hunt
python3 pokedash_cli.py catch pb
python3 pokedash_cli.py dex
python3 pokedash_cli.py stats
python3 pokedash_cli.py store
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

- **Level Bonus**: $1\%$ bonus per level, capped at $25\%$ at Level 25.
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

- **Hunt Attempt**: $+5\text{ XP}$
- **Catch**: Common $+10$, Uncommon $+20$, Rare $+50$, UltraRare $+100$, Epic $+200$, Legendary $+500$, Mythical $+1000\text{ XP}$.
- **New Species Bonus**: Catch XP is **doubled** for registering a new species in your Pokédex!
- **Store Discount**: $0.5\%$ discount per level up to $15\%$ maximum discount.

---

## 🛠️ Architecture

```
pokedash/
├── Cargo.toml            # Rust dependencies & config
├── data/
│   └── pokemon_data.json # All 1025 Pokemon stats, types, rarities
├── static/
│   ├── index.html        # Modern Game Boy/Synthwave Web Dashboard
│   └── sprites/          # Bundled sprites (1.png - 1025.png)
├── src/
│   ├── main.rs           # Entry point
│   ├── models.rs         # Data models and structures
│   ├── game.rs           # Core mechanics, odds, ball formulas
│   ├── storage.rs        # Atomic JSON persistence (~/.pokedash/)
│   ├── service.rs        # Core gameplay service
│   ├── server.rs         # Axum HTTP API & static web router
│   ├── cli.rs            # Clap CLI commands & interactive mode
│   └── tui.rs            # ANSI colored tables & dashboards
└── pokedash_cli.py       # Companion Python CLI with Rich
```
