# ARCHITECTURE.md — PokéDash Engine Architecture & Data Flows

This document details the software architecture, design patterns, and internal component lifecycles for developers and autonomous agents.

---

## 1. Modular Layering

The codebase is organized into four distinct architectural layers with unidirectional dependencies:

```
┌─────────────────────────────────────────────────────────────┐
│                   Presentation Layer                        │
│   static/index.html (GBA LCD UI, Event Loop, Audio, Canvas) │
└──────────────────────────────┬──────────────────────────────┘
                               │ JSON / REST
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                     Transport Layer                         │
│   src/server.rs (Axum Router, Handlers, Middleware, CORS)   │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                      Domain Layer                           │
│   src/service.rs (GameService, Business Logic, Battles)     │
│   src/game.rs    (Catch/XP Math, Type Advantages, Arena)    │
│   src/models.rs  (Entities, Data Transfer Objects)          │
└──────────────────────────────┬──────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Persistence Layer                         │
│   src/storage.rs  (Hybrid Storage: Cloud + Offline Cache)   │
│   src/supabase.rs (PostgREST Client, Table Migrations)      │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. In-Memory Concurrency & Synchronization

- **GameService State**: The core game state is guarded inside an `Arc<RwLock<()>>` inside [`src/service.rs`](src/service.rs):
  ```rust
  pub struct GameService {
      storage: Arc<Storage>,
      lock: Arc<RwLock<()>>,
  }
  ```
- **Read Operations**: Endpoints like `GET /api/player`, `GET /api/wild`, and `GET /api/pokedex` acquire read locks (`self.lock.read().unwrap()`) for fast parallel reads.
- **Mutating Operations**: Operations like `POST /api/hunt`, `POST /api/catch`, and `POST /api/battle/wild-attack` acquire exclusive write locks (`self.lock.write().unwrap()`), mutate state, save changes through [`src/storage.rs`](src/storage.rs), and release the lock.

---

## 3. Storage & Fail-Safe Persistence Flow

The storage subsystem implements a **Write-Through / Read-Fallback** pattern:

```
Save Operation:
1. Write to Supabase cloud table (PostgreSQL PostgREST).
2. Concurrently or sequentially write to local disk (~/.pokedash/*.json).
3. If Supabase fails (offline / invalid credentials), log a warning and proceed with local save.

Load Operation:
1. If Supabase configured: attempt to read from cloud table.
2. If remote record exists, sync/cache to local file and return.
3. If remote fails or is empty, read from local ~/.pokedash/*.json.
4. If local file exists, automatically push/migrate local record to Supabase.
```

---

## 4. Key Performance Optimizations

1. **Pre-Loaded Pokémon Catalog**: All 1025 Pokémon definitions are parsed into memory at startup as a `HashMap<u32, Pokemon>` and indexed into `rarity_pools: HashMap<String, Vec<u32>>` for $O(1)$ random encounter generation.
2. **Procedural Web Audio**: Sound effects use synthesized oscillators (`AudioContext`), minimizing network asset requests.
3. **Optimized SVG Pixel Sprite Sheet**: All 37 retro icons are pre-compiled into a single SVG symbol container (`<symbol id="px-...">`), reducing DOM overhead and eliminating external image round-trips.
