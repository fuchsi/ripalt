CREATE TABLE public.torrent_nfos
(
    id uuid NOT NULL,
    torrent_id uuid NOT NULL,
    data bytea NOT NULL,
    CONSTRAINT torrent_nfos_pkey PRIMARY KEY (id),
    CONSTRAINT torrent_id_key FOREIGN KEY (torrent_id)
        REFERENCES public.torrents (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
TABLESPACE pg_default;

CREATE INDEX torrent_nfos_torrent_id_index
    ON public.torrent_nfos USING btree
    (torrent_id)
    TABLESPACE pg_default;