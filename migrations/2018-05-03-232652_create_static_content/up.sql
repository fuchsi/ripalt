-- DROP TABLE public.static_content;

CREATE TABLE public.static_content
(
    id character varying(100) COLLATE pg_catalog."default" NOT NULL,
    title character varying(100) COLLATE pg_catalog."default" NOT NULL,
    content text COLLATE pg_catalog."default" NOT NULL,
    content_type character varying(32) COLLATE pg_catalog."default" NOT NULL,
    created_at timestamp with time zone NOT NULL,
    updated_at timestamp with time zone NOT NULL,
    CONSTRAINT static_content_pkey PRIMARY KEY (id)
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;