---
name: pokedash-core
description: Core game engine skills, rules, and best practices for PokéDash Advance
rules:
  - no-unicode-emojis
  - pixel-crisp-rendering
  - fail-safe-persistence
  - strict-typing
---

# PokéDash Core Agent Skill

This skill guides agents working on the PokéDash Advance codebase.

## 1. No Unicode Emojis
Never insert Unicode emojis (e.g. 🌾, ⚔️, 💖, 🏆, 📺, 🔊) into HTML templates, Rust response messages, or dynamic JS strings.
Use custom pixel SVG icons instead:
```html
<!-- HTML Example -->
<svg class="pixel-icon"><use href="#px-grass"></use></svg>
```
```js
// JavaScript Example
setDialogue(`${pIcon('fight')} Brock challenges you to a battle!`);
```

## 2. Pixel Crisp Rendering
Ensure all sprites, UI icons, and fonts render with zero anti-aliasing blur:
- CSS: `image-rendering: pixelated;`
- SVG: `shape-rendering: crispEdges;`
- Font: `font-family: var(--pixel-font);` (`PixelEmulator`)

## 3. Fail-Safe Persistence
When implementing storage methods:
- Never panic or throw unhandled errors if Supabase is offline.
- Always implement a write-through fallback to local JSON in `~/.pokedash/`.

## 4. Verification Checklists
- [ ] Run `cargo check` to ensure zero compilation errors.
- [ ] Ensure any newly added icon exists in `static/pixel-icons.svg`.
- [ ] Test that keyboard shortcuts match on-screen UI hints.
