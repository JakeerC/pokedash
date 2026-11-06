import re

with open('static/index.html', 'r', encoding='utf-8') as f:
    html = f.read()

with open('static/pixel-icons.svg', 'r', encoding='utf-8') as f:
    svg_defs = f.read()

# 1. Update Title
html = html.replace(
    '<title>POKÉDASH ADVANCE ⚡ Retro Pokémon Battle & Adventure</title>',
    '<title>POKÉDASH ADVANCE - Retro Pokémon Battle & Adventure</title>'
)

# 2. Add CSS rules in <style>
pixel_css = """
    /* Pixel Icon System */
    .pixel-icon {
      display: inline-block;
      vertical-align: middle;
      width: 14px;
      height: 14px;
      image-rendering: pixelated;
      shape-rendering: crispEdges;
      flex-shrink: 0;
      margin-right: 4px;
    }
    .pixel-icon-no-margin {
      margin-right: 0;
    }
    .pixel-icon-sm {
      width: 12px;
      height: 12px;
      margin-right: 2px;
    }
    .pixel-icon-md {
      width: 16px;
      height: 16px;
      margin-right: 4px;
    }
    .pixel-icon-lg {
      width: 20px;
      height: 20px;
      margin-right: 6px;
    }
    .pixel-icon-nurse {
      width: 56px;
      height: 56px;
      image-rendering: pixelated;
      shape-rendering: crispEdges;
      filter: drop-shadow(0 4px 10px rgba(244, 114, 182, 0.4));
    }
    .pixel-badge-icon {
      width: 18px;
      height: 18px;
      image-rendering: pixelated;
      shape-rendering: crispEdges;
      vertical-align: middle;
      display: inline-block;
    }
    .pixel-dpad-arrow {
      width: 11px;
      height: 11px;
      image-rendering: pixelated;
      shape-rendering: crispEdges;
      display: inline-block;
      vertical-align: middle;
      margin-bottom: 2px;
    }
"""

if '/* Pixel Icon System */' not in html:
    html = html.replace('</style>', f'{pixel_css}\n  </style>', 1)

# Adjust .nurse-joy-avatar styling
old_nurse_css = """.nurse-joy-avatar {
      font-size: 32px;
      margin-bottom: 6px;
      animation: bounce 2s infinite ease-in-out;
    }"""
new_nurse_css = """.nurse-joy-avatar {
      display: flex;
      justify-content: center;
      align-items: center;
      margin-bottom: 8px;
      animation: bounce 2s infinite ease-in-out;
    }"""
if old_nurse_css in html:
    html = html.replace(old_nurse_css, new_nurse_css)

# 3. Insert SVG sprite sheet right after <body class="no-scanlines">
svg_container = f"""\n  <!-- Custom Pixel Icons SVG Sprite Sheet -->\n  <div style="display: none;" id="pixel-icons-container">\n{svg_defs}\n  </div>\n"""
if 'id="pixel-icons-container"' not in html:
    html = html.replace('<body class="no-scanlines">', f'<body class="no-scanlines">{svg_container}', 1)

# 4. Replace Header Switches
html = html.replace(
    '<button id="toggle-sound" class="retro-pill-btn active">🔊 SFX ON</button>',
    '<button id="toggle-sound" class="retro-pill-btn active"><svg class="pixel-icon"><use href="#px-sound-on"></use></svg> SFX ON</button>'
)
html = html.replace(
    '<button id="toggle-scanlines" class="retro-pill-btn">📺 CRT OFF</button>',
    '<button id="toggle-scanlines" class="retro-pill-btn"><svg class="pixel-icon"><use href="#px-crt"></use></svg> CRT OFF</button>'
)
html = html.replace(
    '<button id="toggle-screen-mode" class="retro-pill-btn">⏹ FULL</button>',
    '<button id="toggle-screen-mode" class="retro-pill-btn"><svg class="pixel-icon"><use href="#px-expand"></use></svg> FULL</button>'
)

# 5. Replace Header Trainer & Gold
html = html.replace(
    '<span class="header-tag">👤 <span id="header-trainer-name">TRAINER</span></span>',
    '<span class="header-tag"><svg class="pixel-icon"><use href="#px-trainer"></use></svg> <span id="header-trainer-name">TRAINER</span></span>'
)
html = html.replace(
    'P$ <span id="header-gold-val">0</span>',
    '<svg class="pixel-icon"><use href="#px-coin"></use></svg> P$ <span id="header-gold-val">0</span>'
)

# 6. Replace Nav Tabs
html = html.replace(
    '<button class="nav-tab active" data-tab="hunt">🌾 TALL GRASS</button>',
    '<button class="nav-tab active" data-tab="hunt"><svg class="pixel-icon"><use href="#px-grass"></use></svg> TALL GRASS</button>'
)
html = html.replace(
    '<button class="nav-tab" data-tab="arena">🏟️ ARENA FIGHTS</button>',
    '<button class="nav-tab" data-tab="arena"><svg class="pixel-icon"><use href="#px-arena"></use></svg> ARENA FIGHTS</button>'
)
html = html.replace(
    '<button class="nav-tab" data-tab="center">🏥 POKÉ CENTER</button>',
    '<button class="nav-tab" data-tab="center"><svg class="pixel-icon"><use href="#px-center"></use></svg> POKÉ CENTER</button>'
)
html = html.replace(
    '<button class="nav-tab" data-tab="mart">🏪 POKÉ MART</button>',
    '<button class="nav-tab" data-tab="mart"><svg class="pixel-icon"><use href="#px-mart"></use></svg> POKÉ MART</button>'
)
html = html.replace(
    '<button class="nav-tab" data-tab="dex">📕 POKÉDEX</button>',
    '<button class="nav-tab" data-tab="dex"><svg class="pixel-icon"><use href="#px-dex"></use></svg> POKÉDEX</button>'
)
html = html.replace(
    '<button class="nav-tab" data-tab="trainer">💳 TRAINER CARD</button>',
    '<button class="nav-tab" data-tab="trainer"><svg class="pixel-icon"><use href="#px-card"></use></svg> TRAINER CARD</button>'
)

# 7. Replace Battle Actions
html = html.replace(
    '<span class="dialogue-arrow">▼</span>',
    '<span class="dialogue-arrow"><svg class="pixel-icon pixel-icon-no-margin" style="width:10px; height:10px;"><use href="#px-arrow-down"></use></svg></span>'
)
html = html.replace(
    '<button class="action-menu-btn" id="btn-menu-fight"><span class="k-hint">[1]</span> ⚔️ FIGHT</button>',
    '<button class="action-menu-btn" id="btn-menu-fight"><span class="k-hint">[1]</span> <svg class="pixel-icon"><use href="#px-fight"></use></svg> FIGHT</button>'
)
html = html.replace(
    '<button class="action-menu-btn" id="btn-menu-bag"><span class="k-hint">[2]</span> 🎒 BAG</button>',
    '<button class="action-menu-btn" id="btn-menu-bag"><span class="k-hint">[2]</span> <svg class="pixel-icon"><use href="#px-bag"></use></svg> BAG</button>'
)
html = html.replace(
    '<button class="action-menu-btn" id="btn-menu-pkmn"><span class="k-hint">[3]</span> 🐾 PKMN</button>',
    '<button class="action-menu-btn" id="btn-menu-pkmn"><span class="k-hint">[3]</span> <svg class="pixel-icon"><use href="#px-pkmn"></use></svg> PKMN</button>'
)
html = html.replace(
    '<button class="action-menu-btn" id="btn-menu-run"><span class="k-hint">[4]</span> ⚡ RUN</button>',
    '<button class="action-menu-btn" id="btn-menu-run"><span class="k-hint">[4]</span> <svg class="pixel-icon"><use href="#px-run"></use></svg> RUN</button>'
)

# 8. Replace Hunt Trigger Buttons
html = html.replace(
    '<span class="k-hint" style="color:#facc15;">[SPACE]</span> 🌾 SEARCH TALL GRASS FOR WILD POKÉMON (+5 XP)',
    '<span class="k-hint" style="color:#facc15;">[SPACE]</span> <svg class="pixel-icon"><use href="#px-grass"></use></svg> SEARCH TALL GRASS FOR WILD POKÉMON (+5 XP)'
)
html = html.replace(
    '<span class="k-hint" style="color:#facc15;">[C]</span> 🔊 CRY',
    '<span class="k-hint" style="color:#facc15;">[C]</span> <svg class="pixel-icon"><use href="#px-cry"></use></svg> CRY'
)

# 9. Replace Arena Header & Badge
html = html.replace(
    '<span>🏆 CHAMPION ARENA & GYM BATTLES</span>',
    '<span><svg class="pixel-icon pixel-icon-md"><use href="#px-trophy"></use></svg> CHAMPION ARENA & GYM BATTLES</span>'
)

# 10. Replace Pokémon Center Nurse Joy & Heal HP
html = html.replace(
    '<div class="nurse-joy-avatar">👩‍⚕️</div>',
    '<div class="nurse-joy-avatar"><svg class="pixel-icon-nurse" viewBox="0 0 24 24"><use href="#px-nurse"></use></svg></div>'
)
html = html.replace(
    '<button id="btn-nurse-heal" class="btn-heal-all"><span class="k-hint" style="color:#facc15;">[H]</span> 💖 RESTORE TEAM HP</button>',
    '<button id="btn-nurse-heal" class="btn-heal-all"><span class="k-hint" style="color:#facc15;">[H]</span> <svg class="pixel-icon"><use href="#px-heart"></use></svg> RESTORE TEAM HP</button>'
)

# 11. Replace Mart Shopkeeper & Sell Dupes
html = html.replace(
    '<div style="font-size: 10px; color: #60a5fa; margin-bottom: 4px; font-weight: bold;">🏪 POKÉ MART SHOPKEEPER</div>',
    '<div style="font-size: 10px; color: #60a5fa; margin-bottom: 4px; font-weight: bold;"><svg class="pixel-icon"><use href="#px-mart"></use></svg> POKÉ MART SHOPKEEPER</div>'
)
html = html.replace(
    '💰 SELL DUPES',
    '<svg class="pixel-icon"><use href="#px-coin"></use></svg> SELL DUPES'
)

# 12. Replace D-Pad Arrows
html = html.replace(
    '▲<span class="d-sub">W/↑</span>',
    '<svg class="pixel-dpad-arrow" viewBox="0 0 12 12"><use href="#px-arrow-up"></use></svg><span class="d-sub">W/↑</span>'
)
html = html.replace(
    '◀<span class="d-sub">A/←</span>',
    '<svg class="pixel-dpad-arrow" viewBox="0 0 12 12"><use href="#px-arrow-left"></use></svg><span class="d-sub">A/←</span>'
)
html = html.replace(
    '▶<span class="d-sub">D/→</span>',
    '<svg class="pixel-dpad-arrow" viewBox="0 0 12 12"><use href="#px-arrow-right"></use></svg><span class="d-sub">D/→</span>'
)
html = html.replace(
    '▼<span class="d-sub">S/↓</span>',
    '<svg class="pixel-dpad-arrow" viewBox="0 0 12 12"><use href="#px-arrow-down"></use></svg><span class="d-sub">S/↓</span>'
)

# 13. Add JS helpers and update dynamic JS templates
js_helpers = """
    function pIcon(name, cls = '') {
      return `<svg class="pixel-icon ${cls}" shape-rendering="crispEdges"><use href="#px-${name}"></use></svg>`;
    }

    const BADGE_MAP = {
      'pewter': 'px-badge-boulder',
      'cerulean': 'px-badge-cascade',
      'vermilion': 'px-badge-thunder',
      'celadon': 'px-badge-rainbow',
      'saffron': 'px-badge-marsh',
      'fuchsia': 'px-badge-soul',
      'cinnabar': 'px-badge-volcano',
      'viridian': 'px-badge-earth',
      'indigo': 'px-badge-dragon',
      'battle_tower': 'px-badge-champion',
      'boulder': 'px-badge-boulder',
      'cascade': 'px-badge-cascade',
      'thunder': 'px-badge-thunder',
      'rainbow': 'px-badge-rainbow',
      'marsh': 'px-badge-marsh',
      'soul': 'px-badge-soul',
      'volcano': 'px-badge-volcano',
      'earth': 'px-badge-earth',
      'dragon': 'px-badge-dragon',
      'champion': 'px-badge-champion',
      'Boulder Badge': 'px-badge-boulder',
      'Cascade Badge': 'px-badge-cascade',
      'Thunder Badge': 'px-badge-thunder',
      'Rainbow Badge': 'px-badge-rainbow',
      'Marsh Badge': 'px-badge-marsh',
      'Soul Badge': 'px-badge-soul',
      'Volcano Badge': 'px-badge-volcano',
      'Earth Badge': 'px-badge-earth',
      'Dragon Crest': 'px-badge-dragon',
      'Master Trophy': 'px-badge-champion'
    };

    function renderBadgeIcon(key, cls = 'pixel-badge-icon') {
      const iconId = BADGE_MAP[key] || 'px-trophy';
      return `<svg class="${cls}" shape-rendering="crispEdges"><use href="#${iconId}"></use></svg>`;
    }
"""

if 'function pIcon(' not in html:
    html = html.replace('<script>', f'<script>\n{js_helpers}\n', 1)

# In setDialogue
html = html.replace(
    "el.innerHTML += '<span class=\"dialogue-arrow\">▼</span>';",
    "el.innerHTML += `<span class=\"dialogue-arrow\"><svg class=\"pixel-icon pixel-icon-no-margin\" style=\"width:10px; height:10px;\"><use href=\"#px-arrow-down\"></use></svg></span>`;"
)

# In loadArenaDivisions
html = html.replace(
    '<span class="arena-badge-tag">${d.badge_icon} ${d.badge} ${d.completed ? \'★\' : \'\'}</span>',
    '<span class="arena-badge-tag">${renderBadgeIcon(d.badge_icon || d.id)} ${d.badge} ${d.completed ? pIcon(\'star\', \'pixel-icon-sm\') : \'\'}</span>'
)

# In loadPokemonCenter party lead
html = html.replace(
    "<span>${mon.name.toUpperCase()} ${idx === 0 ? '👑 [LEAD]' : ''}</span>",
    "<span>${mon.name.toUpperCase()} ${idx === 0 ? `${pIcon('crown', 'pixel-icon-sm')} [LEAD]` : ''}</span>"
)

# In showPokemonDetailModal
html = html.replace(
    "👑 MAKE BATTLE LEAD",
    "${pIcon('crown')} MAKE BATTLE LEAD"
)
html = html.replace(
    "🔊 PLAY CRY",
    "${pIcon('cry')} PLAY CRY"
)

# In loadTrainerCard
old_trainer_badges = """      arenaData.divisions.forEach(d => {
        const slot = document.createElement('div');
        slot.className = `badge-slot ${d.completed ? 'unlocked' : ''}`;
        slot.innerHTML = `
          <div>${d.badge_icon}</div>
          <div style="font-size:6px; margin-top:2px;">${d.badge.split(' ')[0]}</div>
        `;
        grid.appendChild(slot);
      });"""

new_trainer_badges = """      arenaData.divisions.forEach(d => {
        const slot = document.createElement('div');
        slot.className = `badge-slot ${d.completed ? 'unlocked' : ''}`;
        slot.innerHTML = `
          <div>${renderBadgeIcon(d.badge_icon || d.id, 'pixel-badge-icon')}</div>
          <div style="font-size:6px; margin-top:2px;">${d.badge.split(' ')[0]}</div>
        `;
        grid.appendChild(slot);
      });"""

if old_trainer_badges in html:
    html = html.replace(old_trainer_badges, new_trainer_badges)

# In button event listeners
html = html.replace(
    "e.target.textContent = soundEnabled ? '🔊 SFX ON' : '🔇 SFX OFF';",
    "e.target.innerHTML = soundEnabled ? `${pIcon('sound-on')} SFX ON` : `${pIcon('sound-off')} SFX OFF`;"
)
html = html.replace(
    "e.target.textContent = isCRTOn ? '📺 CRT ON' : '📺 CRT OFF';",
    "e.target.innerHTML = isCRTOn ? `${pIcon('crt')} CRT ON` : `${pIcon('crt')} CRT OFF`;"
)
html = html.replace(
    "e.target.textContent = '⛶ EXIT';",
    "e.target.innerHTML = `${pIcon('shrink')} EXIT`;"
)
html = html.replace(
    "e.target.textContent = '⏹ FULL';",
    "e.target.innerHTML = `${pIcon('expand')} FULL`;"
)
html = html.replace(
    "btn.textContent = '⛶ EXIT';",
    "btn.innerHTML = `${pIcon('shrink')} EXIT`;"
)
html = html.replace(
    "btn.textContent = '⏹ FULL';",
    "btn.innerHTML = `${pIcon('expand')} FULL`;"
)

with open('static/index.html', 'w', encoding='utf-8') as f:
    f.write(html)

print("Applied pixel icons to static/index.html successfully!")
