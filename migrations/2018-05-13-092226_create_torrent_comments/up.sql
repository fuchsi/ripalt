-- DROP TABLE public.torrent_comments;

CREATE TABLE public.torrent_comments
(
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    torrent_id uuid NOT NULL,
    content text COLLATE pg_catalog."default" NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    updated_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT torrent_comments_pkey PRIMARY KEY (id),
    CONSTRAINT torrent_comments_torrent_id_fkey FOREIGN KEY (torrent_id)
        REFERENCES public.torrents (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT torrent_comments_user_id_fkey FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;