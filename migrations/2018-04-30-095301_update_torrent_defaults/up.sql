ALTER TABLE public.torrents
    ALTER COLUMN visible SET DEFAULT false,
    ALTER COLUMN completed SET DEFAULT 0,
    ALTER COLUMN last_action SET DEFAULT NULL,
    ALTER COLUMN last_seeder SET DEFAULT NULL;