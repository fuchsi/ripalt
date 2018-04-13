-- Table: public.users

-- DROP TABLE public.users;

CREATE TABLE public.users
(
    id uuid NOT NULL,
    name character varying COLLATE pg_catalog."default" NOT NULL,
    email character varying COLLATE pg_catalog."default" NOT NULL,
     password bytea NOT NULL,
    salt bytea NOT NULL,
    status smallint NOT NULL DEFAULT 0,
    created_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    passcode bytea NOT NULL,
    uploaded bigint NOT NULL DEFAULT 0,
    downloaded bigint NOT NULL DEFAULT 0,
    group_id uuid NOT NULL,
    ip_address inet NULL DEFAULT NULL,
    CONSTRAINT users_pkey PRIMARY KEY (id),
    CONSTRAINT users_email_key UNIQUE (email),
    CONSTRAINT users_name_key UNIQUE (name),
    CONSTRAINT users_passcode_key UNIQUE (passcode),
    CONSTRAINT users_group_id_key FOREIGN KEY (group_id)
        REFERENCES public.groups (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE RESTRICT
)
TABLESPACE pg_default;

-- Index: users_email_index

-- DROP INDEX public.users_email_index;

CREATE UNIQUE INDEX users_email_index
    ON public.users USING btree
    (email COLLATE pg_catalog."default")
    TABLESPACE pg_default;

-- Index: users_name_index

-- DROP INDEX public.users_name_index;

CREATE UNIQUE INDEX users_name_index
    ON public.users USING btree
    (name COLLATE pg_catalog."default")
    TABLESPACE pg_default;

-- Index: users_passcode

-- DROP INDEX public.users_passcode;

CREATE UNIQUE INDEX users_passcode
    ON public.users USING btree
    (passcode)
    TABLESPACE pg_default;