ALTER TABLE public.users
    ADD COLUMN last_active timestamp with time zone;

CREATE INDEX users_last_active_key
    ON public.users USING btree
    (last_active ASC NULLS LAST)
    TABLESPACE pg_default;