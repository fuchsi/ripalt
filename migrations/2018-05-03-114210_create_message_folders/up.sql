-- DROP TABLE public.message_folders;

CREATE TABLE public.message_folders
(
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    name character varying(100) COLLATE pg_catalog."default" NOT NULL,
    purge smallint NOT NULL,
    CONSTRAINT message_folders_pkey PRIMARY KEY (id),
    CONSTRAINT message_folders_user_id_name_key UNIQUE (user_id, name),
    CONSTRAINT message_folders_user_id_fkey FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT message_folders_purge_check CHECK (purge >= 0)
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;

WITH folder AS (
    SELECT gen_random_uuid() as id, u.id AS user_id, 'inbox' AS name, 0 as purge FROM public.users u
)
INSERT INTO public.message_folders SELECT * FROM folder;
WITH folder AS (
    SELECT gen_random_uuid() as id, u.id AS user_id, 'sent' AS name, 0 as purge FROM public.users u
)
INSERT INTO public.message_folders SELECT * FROM folder;
WITH folder AS (
    SELECT gen_random_uuid() as id, u.id AS user_id, 'system' AS name, 0 as purge FROM public.users u
)
INSERT INTO public.message_folders SELECT * FROM folder;
