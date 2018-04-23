-- Table: public.transfers

-- DROP TABLE public.transfers;

CREATE TABLE public.transfers
(
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    torrent_id uuid NOT NULL,
    bytes_uploaded bigint NOT NULL,
    bytes_downloaded bigint NOT NULL,
    time_seeded integer NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    completed_at timestamp with time zone,
    CONSTRAINT transfers_pkey PRIMARY KEY (id),
    CONSTRAINT transfers_user_id_torrent_id_key UNIQUE (user_id, torrent_id),
    CONSTRAINT transfers_torrent_id_fkey FOREIGN KEY (torrent_id)
        REFERENCES public.torrents (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT transfers_user_id_fkey FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;


-- Index: transfers_completed_key

-- DROP INDEX public.transfers_completed_key;

CREATE INDEX transfers_completed_key
    ON public.transfers USING btree
    (user_id, completed_at)
    TABLESPACE pg_default;

-- Index: transfers_torrent_id_key

-- DROP INDEX public.transfers_torrent_id_key;

CREATE INDEX transfers_torrent_id_key
    ON public.transfers USING btree
    (torrent_id)
    TABLESPACE pg_default;

-- Index: transfers_user_id_key

-- DROP INDEX public.transfers_user_id_key;

CREATE INDEX transfers_user_id_key
    ON public.transfers USING btree
    (user_id)
    TABLESPACE pg_default;