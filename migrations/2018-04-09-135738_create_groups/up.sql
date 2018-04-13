CREATE TABLE public.groups
(
    id uuid NOT NULL,
    name character varying COLLATE pg_catalog."default" NOT NULL,
    parent_id uuid,
    created_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT groups_pkey PRIMARY KEY (id),
    CONSTRAINT groups_name_key UNIQUE (name),
    CONSTRAINT groups_parent_key FOREIGN KEY (parent_id)
        REFERENCES public.groups (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE RESTRICT
)
TABLESPACE pg_default;
