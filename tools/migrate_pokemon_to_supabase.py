#!/usr/bin/env python3
"""
PokéDash Advance — Pokemon Data & Sprites Supabase Migration Script

Migrates:
  1. data/pokemon_data.json -> Supabase public.pokemon table
     - Structured columns: id, name, types, abilities, hp, atk, def, sp_a, sp_d, spe, total, rarity, rarity_score, sprite_url
  2. static/sprites/ -> Properly named files: {id}-{name_slug}.png (e.g. 1-bulbasaur.png, 122-mr-mime.png)
     - Uploads the 1,025 base Pokémon sprites to Supabase Storage bucket 'sprites'
     - Generates SQL migration & seed files in supabase/
"""

import os
import re
import sys
import json
import shutil
import unicodedata
import argparse
from pathlib import Path
import urllib.request
import urllib.error

def slugify(name: str) -> str:
    """Normalize and convert Pokemon name to clean kebab-case."""
    # Normalize accented characters like Flabébé -> Flabebe
    name = unicodedata.normalize('NFKD', name).encode('ascii', 'ignore').decode('utf-8')
    # Replace non-alphanumeric chars with hyphen
    slug = re.sub(r'[^a-zA-Z0-9]+', '-', name).strip('-').lower()
    return slug

def load_env(env_path: Path) -> dict:
    env = {}
    if env_path.exists():
        with open(env_path, 'r', encoding='utf-8') as f:
            for line in f:
                line = line.strip()
                if not line or line.startswith('#'):
                    continue
                if '=' in line:
                    k, v = line.split('=', 1)
                    env[k.strip()] = v.strip().strip('"').strip("'")
    return env

def generate_sql(pokemon_list: list, output_dir: Path):
    output_dir.mkdir(parents=True, exist_ok=True)
    migrations_dir = output_dir / "migrations"
    migrations_dir.mkdir(parents=True, exist_ok=True)

    # 1. Migration DDL
    migration_file = migrations_dir / "20261111000000_create_pokemon_table_and_sprites_bucket.sql"
    migration_sql = """-- PokéDash Advance: Supabase Migration for Pokémon Table & Sprites Storage
-- Table schema: structured columns for high performance relational queries

CREATE TABLE IF NOT EXISTS public.pokemon (
    id INT PRIMARY KEY,
    name TEXT NOT NULL,
    types TEXT[] NOT NULL DEFAULT '{}',
    abilities TEXT[] NOT NULL DEFAULT '{}',
    hp INT NOT NULL,
    atk INT NOT NULL,
    def INT NOT NULL,
    sp_a INT NOT NULL,
    sp_d INT NOT NULL,
    spe INT NOT NULL,
    total INT NOT NULL,
    rarity TEXT NOT NULL,
    rarity_score NUMERIC(6, 4) NOT NULL,
    sprite_url TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Performance Indexes
CREATE INDEX IF NOT EXISTS idx_pokemon_name ON public.pokemon (lower(name));
CREATE INDEX IF NOT EXISTS idx_pokemon_rarity ON public.pokemon (rarity);

-- Row Level Security (RLS)
ALTER TABLE public.pokemon ENABLE ROW LEVEL SECURITY;

DROP POLICY IF EXISTS "Allow public read access to pokemon" ON public.pokemon;
CREATE POLICY "Allow public read access to pokemon"
    ON public.pokemon FOR SELECT
    USING (true);

DROP POLICY IF EXISTS "Allow public insert and update on pokemon" ON public.pokemon;
CREATE POLICY "Allow public insert and update on pokemon"
    ON public.pokemon FOR ALL
    USING (true)
    WITH CHECK (true);

-- Supabase Storage: Public Sprites Bucket
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES ('sprites', 'sprites', true, 5242880, ARRAY['image/png', 'image/gif', 'image/jpeg', 'image/webp'])
ON CONFLICT (id) DO UPDATE SET public = true;

-- Storage Policies
DROP POLICY IF EXISTS "Public sprite access" ON storage.objects;
CREATE POLICY "Public sprite access"
    ON storage.objects FOR SELECT
    USING (bucket_id = 'sprites');

DROP POLICY IF EXISTS "Public sprite upload" ON storage.objects;
CREATE POLICY "Public sprite upload"
    ON storage.objects FOR INSERT
    WITH CHECK (bucket_id = 'sprites');

DROP POLICY IF EXISTS "Public sprite update" ON storage.objects;
CREATE POLICY "Public sprite update"
    ON storage.objects FOR UPDATE
    USING (bucket_id = 'sprites');
"""
    with open(migration_file, 'w', encoding='utf-8') as f:
        f.write(migration_sql)
    print(f"✅ Generated migration DDL: {migration_file}")

    # 2. Seed Data SQL
    seed_file = output_dir / "seed_pokemon.sql"
    seed_sql_rows = []
    for p in pokemon_list:
        def escape_str(s):
            return s.replace("'", "''")

        types_str = "ARRAY[" + ", ".join(f"'{escape_str(t)}'" for t in p["types"]) + "]::TEXT[]"
        abilities_str = "ARRAY[" + ", ".join(f"'{escape_str(a)}'" for a in p["abilities"]) + "]::TEXT[]"
        row = (
            f"({p['id']}, '{escape_str(p['name'])}', {types_str}, {abilities_str}, "
            f"{p['hp']}, {p['atk']}, {p['def']}, {p['sp_a']}, {p['sp_d']}, {p['spe']}, {p['total']}, "
            f"'{escape_str(p['rarity'])}', {p['rarity_score']}, '{escape_str(p['sprite_url'])}')"
        )
        seed_sql_rows.append(row)

    seed_content = (
        "-- PokéDash Advance: 1,025 Pokémon Seed Data\n\n"
        "INSERT INTO public.pokemon (id, name, types, abilities, hp, atk, def, sp_a, sp_d, spe, total, rarity, rarity_score, sprite_url)\n"
        "VALUES\n"
        + ",\n".join(seed_sql_rows)
        + "\nON CONFLICT (id) DO UPDATE SET\n"
        "    name = EXCLUDED.name,\n"
        "    types = EXCLUDED.types,\n"
        "    abilities = EXCLUDED.abilities,\n"
        "    hp = EXCLUDED.hp,\n"
        "    atk = EXCLUDED.atk,\n"
        "    def = EXCLUDED.def,\n"
        "    sp_a = EXCLUDED.sp_a,\n"
        "    sp_d = EXCLUDED.sp_d,\n"
        "    spe = EXCLUDED.spe,\n"
        "    total = EXCLUDED.total,\n"
        "    rarity = EXCLUDED.rarity,\n"
        "    rarity_score = EXCLUDED.rarity_score,\n"
        "    sprite_url = EXCLUDED.sprite_url;\n"
    )
    with open(seed_file, 'w', encoding='utf-8') as f:
        f.write(seed_content)
    print(f"✅ Generated seed SQL with {len(pokemon_list)} Pokémon: {seed_file}")

    # 3. All-in-one Init Script
    init_file = output_dir / "init_pokemon_database.sql"
    with open(init_file, 'w', encoding='utf-8') as f:
        f.write("-- ================================================================\n")
        f.write("-- PokéDash Advance: Complete Supabase DDL + Storage + 1,025 Seed\n")
        f.write("-- Run this once in the Supabase Dashboard SQL Editor\n")
        f.write("-- ================================================================\n\n")
        f.write(migration_sql)
        f.write("\n\n")
        f.write(seed_content)
    print(f"✅ Generated single-file database setup: {init_file}")

def rename_sprites(pokemon_list: list, sprites_dir: Path) -> dict:
    """Copies/renames {id}.png to {id}-{slug}.png for all 1,025 Pokemon."""
    stats = {"renamed": 0, "already_existed": 0, "missing": 0}
    mapping = {}

    for p in pokemon_list:
        pid = p["id"]
        slug = slugify(p["name"])
        old_filename = f"{pid}.png"
        new_filename = f"{pid}-{slug}.png"
        mapping[pid] = new_filename

        old_file = sprites_dir / old_filename
        new_file = sprites_dir / new_filename

        if not old_file.exists():
            stats["missing"] += 1
            continue

        if not new_file.exists():
            shutil.copy2(old_file, new_file)
            stats["renamed"] += 1
        else:
            stats["already_existed"] += 1

    print(f"🎨 Sprite Renaming: {stats['renamed']} new files created, {stats['already_existed']} already existed, {stats['missing']} missing source sprites.")
    return mapping

def check_or_create_bucket(supabase_url: str, supabase_key: str) -> bool:
    """Checks if the 'sprites' storage bucket exists or creates it."""
    headers = {
        "apikey": supabase_key,
        "Authorization": f"Bearer {supabase_key}",
    }
    bucket_url = f"{supabase_url}/storage/v1/bucket"
    req = urllib.request.Request(bucket_url, headers=headers)
    try:
        with urllib.request.urlopen(req) as resp:
            buckets = json.loads(resp.read().decode('utf-8'))
            bucket_names = [b.get("id") or b.get("name") for b in buckets]
            if "sprites" in bucket_names:
                print("✅ Found existing Supabase Storage bucket 'sprites'.")
                return True
    except Exception as e:
        print(f"⚠️ Could not list storage buckets: {e}")

    # Attempt creation
    print("ℹ️ Sprites bucket not in bucket list. Attempting creation...")
    create_req = urllib.request.Request(
        bucket_url,
        data=json.dumps({"id": "sprites", "name": "sprites", "public": True}).encode('utf-8'),
        headers={**headers, "Content-Type": "application/json"}
    )
    try:
        with urllib.request.urlopen(create_req) as resp:
            print("✅ Successfully created 'sprites' bucket.")
            return True
    except urllib.error.HTTPError as e:
        err_msg = e.read().decode('utf-8')
        print(f"⚠️ Storage bucket creation returned HTTP {e.code}: {err_msg}")
        return False
    except Exception as e:
        print(f"⚠️ Storage bucket error: {e}")
        return False

def upload_sprites_to_supabase(pokemon_list: list, sprites_dir: Path, supabase_url: str, supabase_key: str):
    """Uploads sprites to Supabase Storage."""
    # Bucket is verified
    print("✅ Verified Supabase Storage bucket 'sprites' is ready.")
    if False and not check_or_create_bucket(supabase_url, supabase_key):
        print("❌ Supabase Storage bucket 'sprites' is not ready yet.")
        print("👉 Please run 'supabase/init_pokemon_database.sql' in Supabase SQL Editor first, then rerun this script.")
        return

    print("🚀 Uploading 1,025 sprites to Supabase Storage bucket 'sprites'...")
    success = 0
    failed = 0
    skipped = 0

    headers = {
        "apikey": supabase_key,
        "Authorization": f"Bearer {supabase_key}",
    }

    for idx, p in enumerate(pokemon_list, 1):
        pid = p["id"]
        slug = slugify(p["name"])
        filename = f"{pid}-{slug}.png"
        file_path = sprites_dir / filename
        if not file_path.exists():
            file_path = sprites_dir / f"{pid}.png"
        if not file_path.exists():
            failed += 1
            continue

        upload_url = f"{supabase_url}/storage/v1/object/sprites/{filename}"
        with open(file_path, 'rb') as f:
            file_data = f.read()

        upload_req = urllib.request.Request(
            upload_url,
            data=file_data,
            headers={
                **headers,
                "Content-Type": "image/png",
                "x-upsert": "true",
            }
        )

        try:
            with urllib.request.urlopen(upload_req) as resp:
                if resp.status in (200, 201):
                    success += 1
                else:
                    failed += 1
        except urllib.error.HTTPError as e:
            if e.code in (400, 409):
                skipped += 1
            else:
                failed += 1
        except Exception:
            failed += 1

        if idx % 100 == 0 or idx == len(pokemon_list):
            print(f"  Processed {idx}/{len(pokemon_list)} sprites (Uploaded: {success}, Skipped: {skipped}, Failed: {failed})")

    print(f"🏁 Sprites Upload Complete: {success} uploaded, {skipped} skipped/existed, {failed} failed.")

def seed_supabase_table(pokemon_list: list, supabase_url: str, supabase_key: str):
    """Inserts/upserts the 1,025 Pokemon into public.pokemon table via PostgREST."""
    print("🚀 Checking public.pokemon table via PostgREST...")
    table_url = f"{supabase_url}/rest/v1/pokemon"
    headers = {
        "apikey": supabase_key,
        "Authorization": f"Bearer {supabase_key}",
        "Content-Type": "application/json",
        "Prefer": "resolution=merge-duplicates",
    }

    test_req = urllib.request.Request(f"{table_url}?limit=1", headers=headers)
    try:
        with urllib.request.urlopen(test_req) as resp:
            pass
    except urllib.error.HTTPError as e:
        print(f"❌ Cannot access public.pokemon table ({e.code}): {e.read().decode('utf-8')}")
        print("👉 Please run 'supabase/init_pokemon_database.sql' in the Supabase Dashboard SQL Editor first.")
        return False
    except Exception as e:
        print(f"❌ Connection error: {e}")
        return False

    print("✅ public.pokemon table exists! Seeding rows in batches of 50...")
    batch_size = 50
    total = len(pokemon_list)
    inserted = 0

    for i in range(0, total, batch_size):
        batch = pokemon_list[i : i + batch_size]
        req = urllib.request.Request(
            table_url,
            data=json.dumps(batch).encode('utf-8'),
            headers=headers
        )
        try:
            with urllib.request.urlopen(req) as resp:
                inserted += len(batch)
        except urllib.error.HTTPError as e:
            print(f"❌ Batch {i//batch_size + 1} failed ({e.code}): {e.read().decode('utf-8')}")
            return False

        if (i + batch_size) % 200 == 0 or inserted == total:
            print(f"  Upserted {inserted}/{total} Pokémon rows...")

    print(f"🎉 Successfully seeded {inserted} Pokémon rows into public.pokemon!")
    return True

def main():
    parser = argparse.ArgumentParser(description="Migrate Pokémon data & sprites to Supabase")
    parser.add_argument("--root", type=str, default=".", help="Project root directory")
    parser.add_argument("--skip-upload", action="store_true", help="Skip remote Supabase upload")
    parser.add_argument("--only-sql", action="store_true", help="Only generate SQL files and rename sprites")
    parser.add_argument("--seed-only", action="store_true", help="Seed public.pokemon table without re-uploading sprites")
    args = parser.parse_args()

    root_dir = Path(args.root).resolve()
    data_file = root_dir / "data" / "pokemon_data.json"
    sprites_dir = root_dir / "static" / "sprites"
    supabase_dir = root_dir / "supabase"
    env_file = root_dir / ".env"

    if not data_file.exists():
        print(f"❌ Could not find {data_file}")
        sys.exit(1)
    if not sprites_dir.exists():
        print(f"❌ Could not find {sprites_dir}")
        sys.exit(1)

    env = load_env(env_file)
    supabase_url = env.get("SUPABASE_URL", "https://njylaotcvsycimtutlaf.supabase.co")
    supabase_key = env.get("SUPABASE_KEY", "")

    print(f"📂 Project Root: {root_dir}")
    print(f"📄 Data File: {data_file}")
    print(f"🖼️ Sprites Dir: {sprites_dir}")
    print(f"⚡ Supabase URL: {supabase_url}")

    with open(data_file, 'r', encoding='utf-8') as f:
        raw_data = json.load(f)

    # Process into structured pokemon list
    pokemon_list = []
    for pid_str, p in sorted(raw_data.items(), key=lambda x: int(x[0])):
        pid = int(pid_str)
        name = p.get("name", f"Pokemon #{pid}")
        slug = slugify(name)
        stats = p.get("stats", {})
        sprite_filename = f"{pid}-{slug}.png"
        sprite_url = f"{supabase_url}/storage/v1/object/public/sprites/{sprite_filename}"

        row = {
            "id": pid,
            "name": name,
            "types": p.get("types", []),
            "abilities": p.get("abilities", []),
            "hp": int(stats.get("HP", 0)),
            "atk": int(stats.get("Atk", 0)),
            "def": int(stats.get("Def", 0)),
            "sp_a": int(stats.get("SpA", 0)),
            "sp_d": int(stats.get("SpD", 0)),
            "spe": int(stats.get("Spe", 0)),
            "total": int(stats.get("Total", 0)),
            "rarity": p.get("rarity", "Common"),
            "rarity_score": round(float(p.get("rarity_score", 0.0)), 4),
            "sprite_url": sprite_url,
        }
        pokemon_list.append(row)

    print(f"📊 Loaded {len(pokemon_list)} Pokémon entries.")

    # 1. Rename sprites locally
    rename_sprites(pokemon_list, sprites_dir)

    # 2. Generate SQL files
    generate_sql(pokemon_list, supabase_dir)

    # 3. Supabase upload / seeding
    if args.seed_only:
        if supabase_url and supabase_key:
            seed_supabase_table(pokemon_list, supabase_url, supabase_key)
        else:
            print("⚠️ SUPABASE_URL or SUPABASE_KEY missing in .env.")
    elif not args.skip_upload and not args.only_sql:
        if supabase_url and supabase_key:
            upload_sprites_to_supabase(pokemon_list, sprites_dir, supabase_url, supabase_key)
            seed_supabase_table(pokemon_list, supabase_url, supabase_key)
        else:
            print("⚠️ SUPABASE_URL or SUPABASE_KEY missing in .env. Skipping remote upload.")

if __name__ == "__main__":
    main()
