---
name: pokedash-core
description: Core game engine skills, rules, and best practices for PokéDash Advance
rules:
  - no-unicode-emojis
  - pixel-crisp-rendering
  - fail-safe-persistence
  - strict-typing
---

# PokéDash Core Agent Skill (Gemini Workspace)

This skill guides Gemini and Antigravity agents in developing and maintaining PokéDash Advance.

## Core Rules
1. **Zero Unicode Emojis**: Always use pixel SVG symbols from `static/pixel-icons.svg`.
2. **Pixel Integrity**: Maintain `image-rendering: pixelated;` and integer scaling across all assets.
3. **Rust Concurrency**: Guard state changes with `Arc<RwLock<()>>` in `src/service.rs`.
4. **Cloud + Local Storage**: Support Supabase PostgREST with seamless local fallback to `~/.pokedash/`.
