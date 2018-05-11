-- DROP TABLE public.user_profiles;

CREATE TABLE public.user_profiles
(
    id uuid NOT NULL,
    avatar character varying(100) COLLATE pg_catalog."default" DEFAULT NULL::character varying,
    flair character varying(100) COLLATE pg_catalog."default" DEFAULT NULL::character varying,
    about text COLLATE pg_catalog."default",
    CONSTRAINT user_profiles_pkey PRIMARY KEY (id),
    CONSTRAINT user_profiles_id_fkey FOREIGN KEY (id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;