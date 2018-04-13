-- Table: public.user_properties

-- DROP TABLE public.user_properties;

CREATE TABLE public.user_properties
(
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    name character varying COLLATE pg_catalog."default" NOT NULL,
    value jsonb NOT NULL,
    created_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT user_properties_pkey PRIMARY KEY (id),
    CONSTRAINT user_properties_user_id_name_key UNIQUE (user_id, name),
    CONSTRAINT user_properties_user_id_key FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
TABLESPACE pg_default;

-- Index: user_properties_user_id_index

-- DROP INDEX public.user_properties_user_id_index;

CREATE INDEX user_properties_user_id_index
    ON public.user_properties USING btree
    (user_id)
    TABLESPACE pg_default;

-- Index: user_properties_user_id_name_index

-- DROP INDEX public.user_properties_user_id_name_index;

CREATE UNIQUE INDEX user_properties_user_id_name_index
    ON public.user_properties USING btree
    (user_id, name COLLATE pg_catalog."default")
    TABLESPACE pg_default;