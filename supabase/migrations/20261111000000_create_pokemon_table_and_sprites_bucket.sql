-- PokéDash Advance: Supabase Migration for Pokémon Table & Sprites Storage
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
