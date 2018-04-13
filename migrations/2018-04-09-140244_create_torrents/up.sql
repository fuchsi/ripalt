-- Table: public.torrents

-- DROP TABLE public.torrents;

CREATE TABLE public.torrents
(
    id uuid NOT NULL,
    name character varying(255) COLLATE pg_catalog."default" NOT NULL,
    info_hash bytea NOT NULL,
    category_id uuid NOT NULL,
    user_id uuid,
    description text COLLATE pg_catalog."default" NOT NULL,
    size bigint NOT NULL,
    visible boolean NOT NULL,
    completed integer NOT NULL,
    last_action timestamp with time zone,
    last_seeder timestamp with time zone,
    created_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT torrents_pkey PRIMARY KEY (id),
    CONSTRAINT name_key UNIQUE (name),
    CONSTRAINT info_key UNIQUE (info_hash),
    CONSTRAINT torrents_category_id_fkey FOREIGN KEY (category_id)
        REFERENCES public.categories (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE RESTRICT,
    CONSTRAINT user_id_key FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE SET NULL
)
TABLESPACE pg_default;

-- Index: torrents_category_index

-- DROP INDEX public.torrents_category_index;

CREATE INDEX torrents_category_index
    ON public.torrents USING btree
    (category_id)
    TABLESPACE pg_default;

-- Index: torrents_created_at_index

-- DROP INDEX public.torrents_created_at_index;

CREATE INDEX torrents_created_at_index
    ON public.torrents USING btree
    (created_at)
    TABLESPACE pg_default;

-- Index: torrents_info_hash_index

-- DROP INDEX public.torrents_info_hash_index;

CREATE UNIQUE INDEX torrents_info_hash_index
    ON public.torrents USING btree
    (info_hash)
    TABLESPACE pg_default;

-- Index: torrents_name_index

-- DROP INDEX public.torrents_name_index;

CREATE UNIQUE INDEX torrents_name_index
    ON public.torrents USING btree
    (name COLLATE pg_catalog."default")
    TABLESPACE pg_default;

-- Index: torrents_size_index

-- DROP INDEX public.torrents_size_index;

CREATE INDEX torrents_size_index
    ON public.torrents USING btree
    (size)
    TABLESPACE pg_default;

-- Index: torrents_user_id_index

-- DROP INDEX public.torrents_user_id_index;

CREATE INDEX torrents_user_id_index
    ON public.torrents USING btree
    (user_id)
    TABLESPACE pg_default;

-- Index: torrents_visible_index

-- DROP INDEX public.torrents_visible_index;

CREATE INDEX torrents_visible_index
    ON public.torrents USING btree
    (visible)
    TABLESPACE pg_default;