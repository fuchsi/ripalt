CREATE TABLE public.torrent_files
(
    id uuid NOT NULL,
    torrent_id uuid NOT NULL,
    file_name character varying (255) NOT NULL,
    size bigint NOT NULL,
    CONSTRAINT torrent_files_pkey PRIMARY KEY (id),
    CONSTRAINT torrent_id_key FOREIGN KEY (torrent_id)
        REFERENCES public.torrents (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
TABLESPACE pg_default;

CREATE INDEX torrent_files_torrent_id_index
    ON public.torrent_files USING btree
    (torrent_id)
    TABLESPACE pg_default;

CREATE INDEX torrent_files_torrent_file_name_index
    ON public.torrent_files USING btree
    (file_name)
    TABLESPACE pg_default;