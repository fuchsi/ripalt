DROP INDEX public.users_last_active_key;
ALTER TABLE public.users
    DROP COLUMN last_active;