-- Table: public.acl_user_rules

-- DROP TABLE public.acl_user_rules;

CREATE TABLE public.acl_user_rules
(
    id uuid NOT NULL,
    namespace character varying(100) COLLATE pg_catalog."default" NOT NULL,
    user_id uuid NOT NULL,
    permission acl_permission NOT NULL,
    CONSTRAINT acl_user_rules_pkey PRIMARY KEY (id),
    CONSTRAINT acl_user_rules_namespace_user_id_key UNIQUE (namespace, user_id),
    CONSTRAINT acl_user_rules_user_id_fkey FOREIGN KEY (user_id)
        REFERENCES public.users (id) MATCH SIMPLE
        ON UPDATE NO ACTION
        ON DELETE NO ACTION
)
TABLESPACE pg_default;

-- Index: acl_user_rules_namespae_index

-- DROP INDEX public.acl_user_rules_namespae_index;

CREATE INDEX acl_user_rules_namespae_index
    ON public.acl_user_rules USING btree
    (namespace COLLATE pg_catalog."default")
    TABLESPACE pg_default;

-- Index: acl_user_rules_user_id_index

-- DROP INDEX public.acl_user_rules_user_id_index;

CREATE INDEX acl_user_rules_user_id_index
    ON public.acl_user_rules USING btree
    (user_id)
    TABLESPACE pg_default;