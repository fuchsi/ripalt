CREATE TABLE public.torrent_meta_files
(
    id uuid NOT NULL,
    data bytea NOT NULL,
    CONSTRAINT torrent_meta_files_pkey PRIMARY KEY (id)
)
TABLESPACE pg_default;