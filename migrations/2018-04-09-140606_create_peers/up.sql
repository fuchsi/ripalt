-- Table: public.peers

-- DROP TABLE public.peers;

CREATE TABLE public.peers
(
    id uuid NOT NULL,
    torrent_id uuid NOT NULL,
    user_id uuid NOT NULL,
    ip_address inet NOT NULL,
    port integer NOT NULL,
    bytes_uploaded bigint NOT NULL DEFAULT 0,
    bytes_downloaded bigint NOT NULL DEFAULT 0,
    bytes_left bigint NOT NULL DEFAULT 0,
    seeder boolean NOT NULL DEFAULT false,
    peer_id bytea NOT NULL,
    user_agent character varying(255) COLLATE pg_catalog."default" NOT NULL,
    crypto_enabled boolean NOT NULL DEFAULT false,
    crypto_port integer,
    offset_uploaded bigint NOT NULL DEFAULT 0,
    offset_downloaded bigint NOT NULL DEFAULT 0,
    created_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    finished_at timestamp with time zone,
    updated_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT peers_pkey PRIMARY KEY (id),
    CONSTRAINT peers_tup_key UNIQUE (torrent_id, user_id, peer_id),
    CONSTRAINT peers_torrent_id_fkey FOREIGN KEY (torrent_id)
        REFERENCES public.torrents (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT peers_user_id_fkey FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT peers_port_check CHECK (port > 0 AND port < 65536)
)
TABLESPACE pg_default;

-- Index: peers_ip_address_index

-- DROP INDEX public.peers_ip_address_index;

CREATE INDEX peers_ip_address_index
    ON public.peers USING btree
    (ip_address)
    TABLESPACE pg_default;

-- Index: peers_peer_id_index

-- DROP INDEX public.peers_peer_id_index;

CREATE INDEX peers_peer_id_index
    ON public.peers USING btree
    (peer_id)
    TABLESPACE pg_default;

-- Index: peers_seeder_index

-- DROP INDEX public.peers_seeder_index;

CREATE INDEX peers_seeder_index
    ON public.peers USING btree
    (seeder)
    TABLESPACE pg_default;

-- Index: peers_torrent_id_index

-- DROP INDEX public.peers_torrent_id_index;

CREATE INDEX peers_torrent_id_index
    ON public.peers USING btree
    (torrent_id)
    TABLESPACE pg_default;

-- Index: peers_user_id_index

-- DROP INDEX public.peers_user_id_index;

CREATE INDEX peers_user_id_index
    ON public.peers USING btree
    (user_id)
    TABLESPACE pg_default;