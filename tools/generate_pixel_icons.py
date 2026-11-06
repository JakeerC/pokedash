import os
import sys

def build_svg_symbol(symbol_id, grid_str, palette, width=16, height=16):
    lines = [l.strip() for l in grid_str.strip().splitlines() if l.strip()]
    rects = []
    
    # Merge consecutive identical pixels horizontally
    for y, line in enumerate(lines):
        x = 0
        while x < len(line):
            char = line[x]
            if char in palette and palette[char] is not None:
                color = palette[char]
                # count consecutive identical chars
                run_len = 1
                while (x + run_len) < len(line) and line[x + run_len] == char:
                    run_len += 1
                rects.append(f'<rect x="{x}" y="{y}" width="{run_len}" height="1" fill="{color}"/>')
                x += run_len
            else:
                x += 1
                
    inner_svg = "".join(rects)
    return f'<symbol id="{symbol_id}" viewBox="0 0 {width} {height}">{inner_svg}</symbol>'

# Define palettes and grids
ICONS = {}

# 1. px-grass: Tall Wild Grass
ICONS['px-grass'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0a2318', # dark forest edge
        'd': '#15803d', # dark green
        'm': '#22c55e', # emerald green
        'l': '#86efac', # lime highlight
    },
    'grid': """
................
...#......#.....
..#l#....#l#..#.
..#ml#..#ml#.#l#
..#mm#..#mm#.#m#
.#dmm#..#mmm#mm#
.#dmm#..#dmmmmd#
.#dmmm##dmmmmm##
.#dmmmmdmmmmmmd#
.##dmmmmmmmmmmd#
.##dmmmmmmmdmmd#
..##ddmmmmdddd##
...###ddddddd##.
.....#########..
................
................
"""
}

# 2. px-arena: Arena Colosseum / Stadium
ICONS['px-arena'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'y': '#facc15', # gold dome
        'Y': '#ca8a04', # dark gold
        'w': '#cbd5e1', # stone pillar
        'W': '#94a3b8', # shadow stone
        'd': '#1e293b', # dark arch opening
    },
    'grid': """
................
.....######.....
...##yyyyyy##...
..#yyyyyyyyYY#..
.##############.
.#wwwwwwwwwwww#.
.#wW#dd#wW#dd#w#.
.#wW#dd#wW#dd#w#.
.#wW#dd#wW#dd#w#.
.#wwwwwwwwwwww#.
.#wW##########w#.
.#wW#dddddddd#w#.
.###dddddddddd###
.##############.
................
................
"""
}

# 3. px-center: Poké Center Red Cross / Building
ICONS['px-center'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#1e293b',
        'r': '#ef4444', # roof red
        'R': '#b91c1c', # dark red
        'w': '#f8fafc', # white wall
        'W': '#e2e8f0', # shaded wall
        'c': '#38bdf8', # cyan glass
        'C': '#0284c7', # dark cyan
    },
    'grid': """
................
....########....
...#rrrrrrrr#...
..#rrrrrrrrRR#..
.##############.
.#wwwwwwwwwwww#.
.#wwW#rr#wWwww#.
.#wwW#rr#wWwww#.
.#w######wwwww#.
.#wwW#rr#wWwww#.
.#wwW#rr#wWwww#.
.#wwwwwwwwwwww#.
.#wwW########w#.
.###ccC####ccC##
...#ccC#..#ccC#.
...#####..#####.
"""
}

# 4. px-mart: Poké Mart Blue Shop Awning
ICONS['px-mart'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'b': '#3b82f6', # blue awning
        'B': '#1d4ed8', # dark blue
        'w': '#f8fafc', # white stripe
        'W': '#cbd5e1',
        'c': '#38bdf8', # window glass
        'd': '#d97706', # shop door
    },
    'grid': """
................
...##########...
..#bbbbbbbbBB#..
.##############.
.#bbwwbbwwbbww#.
.#bbwwbbwwbbww#.
.#BBWWBBWWBBWW#.
.##############.
.#wwwwwwwwwwww#.
.#w#cc#ww#dd#w#.
.#w#cc#ww#dd#w#.
.#w####ww#dd#w#.
.#wwwwwwwwwwww#.
.##############.
................
................
"""
}

# 5. px-dex: Red Pokédex
ICONS['px-dex'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#450a0a',
        'r': '#ef4444',
        'R': '#991b1b',
        'b': '#38bdf8', # sensor light
        'B': '#e0f2fe', # glint
        'y': '#facc15', # yellow led
        'g': '#22c55e', # green led
        's': '#0284c7', # screen dark blue
        'S': '#38bdf8', # radar line
        'w': '#94a3b8', # metal bezel
    },
    'grid': """
................
.############...
.#Bb#r#y#g###...
.#bb#r#y#g#rR#..
.####r#r#r#rR#..
.#rrrrrrrrrrR#..
.############...
.#w########rr#..
.#w#ssssss#rr#..
.#w#sSSsss#rr#..
.#w#ssssss#rr#..
.#w########rr#..
.#rrrrrrrrrrR#..
.#rrrrrrrrrrR#..
.############...
................
"""
}

# 6. px-card: Trainer Card / Passport
ICONS['px-card'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'g': '#facc15', # gold trim
        'w': '#f8fafc', # card face
        'W': '#e2e8f0', # shading
        'p': '#f43f5e', # portrait
        'b': '#3b82f6', # rank chip
        's': '#64748b', # stripe
    },
    'grid': """
................
.##############.
.#gggggggggggg#.
.#g##########g#.
.#g#pp#bbbbwWg#.
.#g#pp#wwwwwwg#.
.#g####ssssssg#.
.#gwwwwwwwwwwg#.
.#g##########g#.
.#gwwwwwwwwwwg#.
.#g##########g#.
.#gWWWWWWWWWWg#.
.#gggggggggggg#.
.##############.
................
................
"""
}

# 7. px-trainer: Trainer Cap & Avatar
ICONS['px-trainer'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'r': '#ef4444', # red cap
        'R': '#991b1b',
        'w': '#ffffff', # Poké symbol on cap
        's': '#fed7aa', # skin tone
        'e': '#0f172a', # eye
        'h': '#451a03', # brown hair
        'b': '#2563eb', # blue vest
    },
    'grid': """
................
....########....
...#rrrrrrrr#...
..#rrrrwrrrrr#..
..#rrrwwwrrrr#..
..#rrrrrrrrrR#..
.##############.
...#h#ssss#h#...
...#h#sesh#h#...
....#ssssss#....
....#ssssss#....
...###bbbb###...
..#bbb#bb#bbb#..
..#bbb####bbb#..
.##############.
................
"""
}

# 8. px-fight: Crossed Broadswords
ICONS['px-fight'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        's': '#f8fafc', # steel blade
        'S': '#94a3b8', # blade shadow
        'g': '#facc15', # gold guard
        'G': '#ca8a04',
        'r': '#ef4444', # ruby pommel
    },
    'grid': """
.#............#.
.##..........##.
.#s#...##...#s#.
..#s#.#sS#.#s#..
...#s##ss##s#...
....#sSssSs#....
.....#ssss#.....
....##gggg##....
...#gr#GG#rg#...
..#Gg#....#Gg#..
.#Gg#......#Gg#.
.##..........##.
................
................
................
................
"""
}

# 9. px-bag: Adventurer Rucksack
ICONS['px-bag'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#271004',
        'l': '#b45309', # leather
        'L': '#78350f', # dark leather
        'h': '#d97706', # light leather
        'y': '#facc15', # brass buckle
        'b': '#0284c7', # strap
    },
    'grid': """
................
....########....
...#bbbbbbbb#...
..#bbbbbbbbbb#..
.##############.
.#hhhhhhhlllll#.
.#h#yy#h#yy#ll#.
.#hhhhhhhlllll#.
.##############.
.#hhhhhhhlllll#.
.#hh########ll#.
.#hh#yyyyyy#ll#.
.#hh#yyyyyy#ll#.
.#hh########ll#.
.##############.
................
"""
}

# 10. px-pkmn: Classic Pokéball
ICONS['px-pkmn'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'r': '#ef4444', # red top
        'R': '#dc2626',
        'w': '#f8fafc', # white bottom
        'W': '#cbd5e1',
        'b': '#0f172a', # band
    },
    'grid': """
................
.....######.....
...##rrrrrr##...
..#rrrrrrrrrr#..
.#rrrrrrrrrrrr#.
.#rrrrrrrrrrrr#.
.######ww######.
.#bbbb#ww#bbbb#.
.######ww######.
.#wwwwwwwwwwww#.
.#wwwwwwwwwwWW#.
..#wwwwwwwwWW#..
...##wwwwWW##...
.....######.....
................
................
"""
}

# 11. px-run: Winged Speed Sneaker
ICONS['px-run'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'c': '#06b6d4', # cyan shoe
        'C': '#0891b2',
        'w': '#f8fafc', # white wing
        'W': '#cbd5e1',
        'y': '#facc15', # speed line
    },
    'grid': """
................
...#............
..#w#...###.....
.#ww#..#ccc#....
#wwW#.#ccccc#...
.###..#cccccc#..
.....#cccccccc#.
.###.#cccccccc#.
#yyy##cccccccc#.
#yyy#ccccccccc#.
#yyy#ccccccyyy#.
.##############.
.#wwwwwwwwwwww#.
.#WWWWWWWWWWWW#.
.##############.
................
"""
}

# 12. px-sound-on: Speaker with Waves
ICONS['px-sound-on'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        's': '#64748b', # speaker body
        'S': '#475569',
        'g': '#22c55e', # sound waves
    },
    'grid': """
................
...#............
..##...#...#....
.###....#...#...
#####...#...#...
#sss#..#g#.#g#..
#sss#..#g#.#g#..
#sss#..#g#.#g#..
#sss#..#g#.#g#..
#####...#...#...
.###....#...#...
..##...#...#....
...#............
................
................
................
"""
}

# 13. px-sound-off: Speaker Muted with Red Slash
ICONS['px-sound-off'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        's': '#64748b',
        'r': '#ef4444', # red slash
    },
    'grid': """
................
...#.........#r#
..##........#r#.
.###.......#r#..
#####.....#r#...
#sss#....#r#....
#sss#...#r#.....
#sss#..#r#......
#sss#.#r#.......
#####r#.........
.###r#..........
..##r#..........
.#r#............
................
................
................
"""
}

# 14. px-crt: Retro CRT Monitor
ICONS['px-crt'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'b': '#334155', # bezel
        'd': '#0f172a', # screen bg
        'g': '#22c55e', # green phosphor scanline
        'w': '#f8fafc',
    },
    'grid': """
................
.##############.
.#bbbbbbbbbbbb#.
.#b##########b#.
.#b#dddddddd#b#.
.#b#gggggggg#b#.
.#b#dddddddd#b#.
.#b#gggggggg#b#.
.#b##########b#.
.#bbbbbbbbbbbb#.
.##############.
.....######.....
.....######.....
...##########...
................
................
"""
}

# 15. px-expand: Fullscreen Maximize Bracket
ICONS['px-expand'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'c': '#38bdf8', # cyan corners
    },
    'grid': """
................
.######..######.
.#cccc#..#cccc#.
.#c####..####c#.
.#c#........#c#.
.#c#........#c#.
.###........###.
................
................
.###........###.
.#c#........#c#.
.#c#........#c#.
.#c####..####c#.
.#cccc#..#cccc#.
.######..######.
................
"""
}

# 16. px-shrink: Exit Fullscreen Bracket
ICONS['px-shrink'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'c': '#f43f5e', # red/rose collapse
    },
    'grid': """
................
....###..###....
....#c#..#c#....
.####c#..#c####.
.#cccc#..#cccc#.
.######..######.
................
................
.######..######.
.#cccc#..#cccc#.
.####c#..#c####.
....#c#..#c####.
....###..###....
................
................
................
"""
}

# 17. px-cry: Audio Cry Megaphone
ICONS['px-cry'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        's': '#475569',
        'w': '#cbd5e1',
        'y': '#facc15',
    },
    'grid': """
................
.....#..........
....##..#...#...
...#s#...#...#..
..#ss#...#...#..
.#####..#y#.#y#.
.#www#..#y#.#y#.
.#www#..#y#.#y#.
.#####..#y#.#y#.
..#ss#...#...#..
...#s#...#...#..
....##..#...#...
.....#..........
................
................
................
"""
}

# 18. px-trophy: Champion Trophy Cup
ICONS['px-trophy'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#713f12',
        'y': '#facc15', # gold
        'Y': '#ca8a04', # gold shade
        'w': '#fef08a', # highlight
        's': '#ef4444', # center star/gem
    },
    'grid': """
................
.##############.
.#yyyyyyyyyyyy#.
.#y#yyyyyyyy#Y#.
.#y#yyw##wyy#Y#.
.###yyssssyY###.
..##yyw##wyY##..
...##yyyyyy##...
....##yyyy##....
......#yy#......
......#yy#......
.....######.....
....#yyyyyy#....
...############.
................
................
"""
}

# 19. px-heart: 8-Bit Healing Heart
ICONS['px-heart'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#881337',
        'r': '#f43f5e',
        'R': '#e11d48',
        'w': '#ffe4e6',
    },
    'grid': """
................
..####....####..
.#rrrr#..#rrrr#.
#rrwwrr##rrrrRR#
#rrwwrrrrrrrrRR#
#rrrrrrrrrrrrRR#
.#rrrrrrrrrrRR#.
..#rrrrrrrrRR#..
...#rrrrrrRR#...
....#rrrrRR#....
.....#rrRR#.....
......#RR#......
.......##.......
................
................
................
"""
}

# 20. px-coin: Gold Poké Dollar Coin
ICONS['px-coin'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#78350f',
        'y': '#facc15',
        'Y': '#ca8a04',
        'p': '#78350f', # 'P' stamp
        'w': '#fef08a',
    },
    'grid': """
.....######.....
...##yyyyyy##...
..#yyyyyyyyYY#..
.#yy########YY#.
.#yy#ppppp#wYY#.
.#yy#pp#pp#wYY#.
.#yy#ppppp#wYY#.
.#yy#pp####wYY#.
.#yy#pp#yyyyYY#.
.#yy#pp#yyyyYY#.
.#yy####yyyyYY#.
..#yyyyyyyyYY#..
...##yyyyYY##...
.....######.....
................
................
"""
}

# 21. px-crown: Royal Battle Lead Crown
ICONS['px-crown'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#713f12',
        'y': '#facc15',
        'Y': '#ca8a04',
        'r': '#ef4444', # ruby jewels
        'w': '#fef08a',
    },
    'grid': """
................
.#....#....#....
.#r#..#r#..#r#..
.###..###..###..
.#y#..#y#..#y#..
.#yy##yyy##yy#..
.#wywyywywyyY#..
.#yyyyyyyyyyY#..
.#############..
.#y#r#y#r#y#rY#.
.#yyyyyyyyyyY#..
.#############..
................
................
................
................
"""
}

# 22. px-star: Completed 5-Point Star
ICONS['px-star'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#713f12',
        'y': '#facc15',
        'w': '#fef08a',
        'Y': '#ca8a04',
    },
    'grid': """
.......##.......
......#yy#......
......#ww#......
....##yyyy##....
...#yyyyyyyy#...
.##yyyyyyyyyy##.
.#yyyyyyyyyyyy#.
..##yyyyyyyy##..
...#yyyyyyyy#...
...#yyyyyyyy#...
..#yyyyyyyyyy#..
..#yy##yy##yy#..
.##y#..##..#y##.
.###........###.
................
................
"""
}

# 23. px-arrow-down: Dialogue Arrow
ICONS['px-arrow-down'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0f172a',
        'y': '#facc15',
        'w': '#fef08a',
    },
    'grid': """
................
................
................
.##############.
.#wwwwwwwwwwww#.
..#yyyyyyyyyy#..
...#yyyyyyyy#...
....#yyyyyy#....
.....#yyyy#.....
......#yy#......
.......##.......
................
................
................
................
................
"""
}

# 24. 10 GYM BADGES:
# A. px-badge-boulder: Boulder Badge (faceted gray stone)
ICONS['px-badge-boulder'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#1e293b',
        's': '#94a3b8',
        'S': '#64748b',
        'w': '#e2e8f0',
    },
    'grid': """
.....######.....
...##wwwwww##...
..#wwwwwwwwww#..
.#wwww#ssssssS#.
.#www#sssssssS#.
.#ww#ssssssssS#.
.#w#sssssssssS#.
.#w#sssssssssS#.
.#w#sssssssssS#.
.#w#sssssssssS#.
.#w#SSSSSSSSSS#.
..#SSSSSSSSSS#..
...##SSSSSS##...
.....######.....
................
................
"""
}

# B. px-badge-cascade: Cascade Badge (cerulean water droplet)
ICONS['px-badge-cascade'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#0c4a6e',
        'c': '#38bdf8',
        'C': '#0284c7',
        'w': '#e0f2fe',
    },
    'grid': """
.......##.......
......#cc#......
.....#cccc#.....
....#cccccc#....
...#ccc##ccc#...
...#cc#ww#cc#...
..#cccc##cccc#..
..#cccccccccc#..
.#cccccccccccc#.
.#cccccccccccc#.
.#cccccccccccc#.
..#cccccccccc#..
...##CCCCCC##...
.....######.....
................
................
"""
}

# C. px-badge-thunder: Thunder Badge (electric 8-point gold sunburst)
ICONS['px-badge-thunder'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#713f12',
        'y': '#facc15',
        'w': '#fef08a',
        'Y': '#ca8a04',
        'r': '#ef4444', # center ruby
    },
    'grid': """
.......##.......
...#..#yy#..#...
..#y##yyyy##y#..
...#yyyyyyyy#...
.##yyyyyyyyyy##.
.#yyyyyyyyyyyy#.
.#yyyy#rr#yyyy#.
.#####rrrr#####.
.#yyyy#rr#yyyy#.
.#yyyyyyyyyyyy#.
.##yyyyyyyyyy##.
...#yyyyyyyy#...
..#y##yyyy##y#..
...#..#yy#..#...
.......##.......
................
"""
}

# D. px-badge-rainbow: Rainbow Badge (flower blossom with rainbow petals)
ICONS['px-badge-rainbow'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#1e293b',
        'r': '#ef4444', # red petal
        'o': '#f97316', # orange petal
        'y': '#facc15', # yellow petal
        'g': '#22c55e', # green petal
        'c': '#06b6d4', # cyan petal
        'b': '#3b82f6', # blue petal
        'p': '#a855f7', # purple petal
        'w': '#ffffff', # center pistil
    },
    'grid': """
.....######.....
...##rrrrrr##...
..#oo#rrrr#yy#..
.#oooo#rr#yyyy#.
.#oooo#rr#yyyy#.
.##oo######yy##.
.#pp#wwwwww#gg#.
.#pp#wwwwww#gg#.
.##bb######cc##.
.#bbbb#bb#cccc#.
.#bbbb#bb#cccc#.
..#bb#bbbb#cc#..
...##bbbbbb##...
.....######.....
................
................
"""
}

# E. px-badge-marsh: Marsh Badge (golden concentric circular disk)
ICONS['px-badge-marsh'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#713f12',
        'y': '#facc15',
        'Y': '#ca8a04',
        'w': '#fef08a',
        'd': '#451a03',
    },
    'grid': """
.....######.....
...##yyyyyy##...
..#yyyyyyyyyy#..
.#yy########yy#.
.#y#dddddddd#y#.
.#y#d######d#y#.
.#y#d#yyyy#d#y#.
.#y#d#ywwy#d#y#.
.#y#d#yyyy#d#y#.
.#y#d######d#y#.
.#y#dddddddd#y#.
.#yy########yy#.
..#yyyyyyyyyy#..
...##yyyyyy##...
.....######.....
................
"""
}

# F. px-badge-soul: Soul Badge (ninja heart jewel in vivid purple)
ICONS['px-badge-soul'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#3b0764',
        'p': '#c084fc',
        'P': '#7e22ce',
        'w': '#f3e8ff',
    },
    'grid': """
..####....####..
.#pppp#..#pppp#.
#ppwwpp##ppppPP#
#ppwwppppppppPP#
#ppppppppppppPP#
.#ppppppppppPP#.
..#ppppppppPP#..
...#ppppppPP#...
....#ppppPP#....
.....#ppPP#.....
......#PP#......
.......##.......
................
................
................
................
"""
}

# G. px-badge-volcano: Volcano Badge (fiery flame teardrop)
ICONS['px-badge-volcano'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#7c2d12',
        'r': '#ef4444', # red flame
        'o': '#f97316', # orange flame
        'y': '#facc15', # yellow center
        'w': '#fef08a',
    },
    'grid': """
........##......
.......#rr#.....
......#rrrr#....
.....#rroorr#...
....#rroooyrr#..
...#rrooyyyyrr#.
...#rooyyyyyro#.
..#rrooyywyyro#.
..#rrooyyyyyro#.
..#rrooyyyyroo#.
...#rrooooorr#..
....#rrrrrrrr#..
.....##rrrr##...
.......####.....
................
................
"""
}

# H. px-badge-earth: Earth Badge (emerald green leaf/feather)
ICONS['px-badge-earth'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#064e3b',
        'g': '#10b981',
        'G': '#047857',
        'l': '#6ee7b7',
        'y': '#facc15',
    },
    'grid': """
.......##.......
......#gg#......
.....#gggg#.....
....#glgglG#....
...#gglgglGG#...
..#ggglgglGGG#..
..#ggglgglGGG#..
.#gggglyylGGGG#.
.#gggglyylGGGG#.
..#ggglgglGGG#..
..#ggglgglGGG#..
...#gglgglGG#...
....#glgglG#....
.....#gggg#.....
......#GG#......
.......##.......
"""
}

# I. px-badge-dragon: Dragon Crest (gold & scarlet dragon wings)
ICONS['px-badge-dragon'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#450a0a',
        'r': '#dc2626',
        'y': '#facc15',
        'Y': '#ca8a04',
        'w': '#fef08a',
    },
    'grid': """
#..............#
##............##
#y#..........#y#
#yy##......##yy#
.#yyy#....#yyy#.
..#yyy#..#yyy#..
...#yyy##yyy#...
....#yyyyyy#....
...##yyrryy##...
..#yyrrrrrryy#..
.#yyrrrrrrrryy#.
.#yrrrrrrrrrry#.
.##rr####rr##...
..###......###..
................
................
"""
}

# J. px-badge-champion: Master Grand Trophy Badge
ICONS['px-badge-champion'] = {
    'width': 16, 'height': 16,
    'palette': {
        '.': None,
        '#': '#451a03',
        'y': '#facc15',
        'Y': '#ca8a04',
        'w': '#fef08a',
        'd': '#38bdf8', # diamond jewel
        'D': '#0284c7',
    },
    'grid': """
.##############.
.#yyyyyyyyyyyy#.
.#y#yyyyyyyy#Y#.
.#y#yyyddyyy#Y#.
.###yyddddyy###.
..##yyyddyyy##..
...##yyyyyy##...
....##yyyy##....
......#yy#......
.....######.....
....#dddddd#....
...#dDDDDDDd#...
...############.
................
................
................
"""
}

# 25. px-nurse: 24x24 Retro Nurse Joy Portrait
ICONS['px-nurse'] = {
    'width': 24, 'height': 24,
    'palette': {
        '.': None,
        '#': '#1e1b4b',
        'p': '#f472b6', # pink hair
        'P': '#db2777', # dark pink hair
        'w': '#f8fafc', # white cap
        'W': '#cbd5e1',
        'r': '#ef4444', # red cross on cap
        's': '#fed7aa', # face skin tone
        'e': '#0284c7', # blue eyes
        'b': '#38bdf8', # blue ribbon / tie
        'c': '#f8fafc', # white collar
    },
    'grid': """
.........########.......
.......##wwwwwwww##.....
......#wwwwwwwwwwww#....
.....#wwww#rr#wwwwww#...
.....#www######wwwww#...
.....#wwww#rr#wwwwww#...
.....#wwwwwwwwwwwwww#...
.....################...
...##pppppppppppppppp##.
..#pppppppppppppppppppp#
.#pppppp##pppppp##pppppp#
#pppppp#ss#pppp#ss#ppppp#
#pppppp#se#pppp#se#ppppp#
#pppppp#ss#pppp#ss#ppppp#
#PPPPpp#ssssssssss#ppPPP#
.#PPPpp#ssssssssss#ppPPP#
..#PPppp#ssssssss#pppPP#.
...##ppp#ssssssss#ppp##.
.....###ssssssssss###...
......#cc#bbbbbb#cc#....
.....#cccc#bbbb#cccc#...
....#cccccc####cccccc#..
...#cccccccccccccccccc#.
...####################.
"""
}

def generate_all_symbols():
    symbols = []
    for icon_id, data in ICONS.items():
        sym = build_svg_symbol(icon_id, data['grid'], data['palette'], data['width'], data['height'])
        symbols.append(sym)
    return "\n".join(symbols)

ICONS['px-arrow-up'] = {
    'width': 12, 'height': 12,
    'palette': { '.': None, '#': '#1e293b', 'w': '#f8fafc', 's': '#94a3b8' },
    'grid': """
............
.....##.....
....#ww#....
...#wwww#...
..#wwwwww#..
.#wwwwwwww#.
#wwwwwwwwww#
#####ww#####
....#ww#....
....#ww#....
....####....
............
"""
}

ICONS['px-arrow-down'] = {
    'width': 12, 'height': 12,
    'palette': { '.': None, '#': '#1e293b', 'w': '#f8fafc', 's': '#94a3b8' },
    'grid': """
............
....####....
....#ww#....
....#ww#....
#####ww#####
#wwwwwwwwww#
.#wwwwwwww#.
..#wwwwww#..
...#wwww#...
....#ww#....
.....##.....
............
"""
}

ICONS['px-arrow-left'] = {
    'width': 12, 'height': 12,
    'palette': { '.': None, '#': '#1e293b', 'w': '#f8fafc', 's': '#94a3b8' },
    'grid': """
............
......#.....
.....##.....
....#w#.....
...#ww######
..#wwwwwwwww
.#wwwwwwwwww
..#wwwwwwwww
...#ww######
....#w#.....
.....##.....
......#.....
"""
}

ICONS['px-arrow-right'] = {
    'width': 12, 'height': 12,
    'palette': { '.': None, '#': '#1e293b', 'w': '#f8fafc', 's': '#94a3b8' },
    'grid': """
............
.....#......
.....##.....
.....#w#....
######ww#...
wwwwwwwww#..
wwwwwwwwww#.
wwwwwwwww#..
######ww#...
.....#w#....
.....##.....
.....#......
"""
}

if __name__ == '__main__':
    content = generate_all_symbols()
    print(f"Generated {len(ICONS)} pixel icon symbols!")
    with open('static/pixel-icons.svg', 'w') as f:
        f.write(f'<svg xmlns="http://www.w3.org/2000/svg" style="display: none;">\n<defs>\n{content}\n</defs>\n</svg>')
    print("Saved to static/pixel-icons.svg")
