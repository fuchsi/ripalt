-- Table: public.messages

-- DROP TABLE public.messages;

CREATE TABLE public.messages
(
    id uuid NOT NULL,
    folder_id uuid NOT NULL,
    sender_id uuid,
    receiver_id uuid NOT NULL,
    subject character varying(255) COLLATE pg_catalog."default" NOT NULL,
    body text COLLATE pg_catalog."default" NOT NULL,
    is_read boolean NOT NULL DEFAULT false,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT messages_pkey PRIMARY KEY (id),
    CONSTRAINT messages_folder_id_fkey FOREIGN KEY (folder_id)
        REFERENCES public.message_folders (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT messages_receiver_id_fkey FOREIGN KEY (receiver_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE,
    CONSTRAINT messages_sender_id_fkey FOREIGN KEY (sender_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE SET NULL
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;

-- Index: messages_folder_id_read_key

-- DROP INDEX public.messages_folder_id_read_key;

CREATE INDEX messages_folder_id_read_key
    ON public.messages USING btree
    (folder_id, is_read)
    TABLESPACE pg_default;