-- DROP TABLE public.torrent_images;

CREATE TABLE public.torrent_images
(
    id uuid NOT NULL,
    torrent_id uuid NOT NULL,
    file_name character varying(255) COLLATE pg_catalog."default" NOT NULL,
    index smallint NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT torrent_images_pkey PRIMARY KEY (id),
    CONSTRAINT torrent_images_torrent_id_file_name_key UNIQUE (torrent_id, file_name),
    CONSTRAINT torrent_images_torrent_id_index_key UNIQUE (torrent_id, index),
    CONSTRAINT torrent_images_torrent_id_fkey FOREIGN KEY (torrent_id)
        REFERENCES public.torrents (id) MATCH SIMPLE
        ON UPDATE NO CASCADE
        ON DELETE NO CASCADE,
    CONSTRAINT torrent_images_index_check CHECK (index >= 0)
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;