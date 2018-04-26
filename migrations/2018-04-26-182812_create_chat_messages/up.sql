-- Your SQL goes here
CREATE TABLE "public".chat_messages (
    id uuid NOT NULL,
    user_id uuid NOT NULL,
    chat smallint NOT NULL DEFAULT 0,
    message text NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT chat_messages_pkey PRIMARY KEY (id),
    CONSTRAINT chat_messages_user_id_fkey FOREIGN KEY (user_id)
        REFERENCES "public".users (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
WITH (
    OIDS = FALSE
)
TABLESPACE pg_default;

CREATE INDEX chat_messages_chat_key
    ON "public".chat_messages USING btree
    (chat)
    TABLESPACE pg_default;

CREATE INDEX chat_messages_created_at_key
    ON "public".chat_messages USING btree
    (created_at)
    TABLESPACE pg_default;