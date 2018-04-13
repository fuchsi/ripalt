-- Table: public.acl_group_rules

-- DROP TABLE public.acl_group_rules;

CREATE TYPE public.acl_permission AS ENUM
    ('none', 'read', 'write', 'create', 'delete');

CREATE TABLE public.acl_group_rules
(
    id uuid NOT NULL,
    namespace character varying(100) COLLATE pg_catalog."default" NOT NULL,
    group_id uuid NOT NULL,
    permission acl_permission NOT NULL,
    CONSTRAINT acl_group_rules_pkey PRIMARY KEY (id),
    CONSTRAINT acl_group_rules_namespace_group_id_key UNIQUE (namespace, group_id),
    CONSTRAINT acl_group_rules_group_id_fkey FOREIGN KEY (group_id)
        REFERENCES public.groups (id) MATCH SIMPLE
        ON UPDATE CASCADE
        ON DELETE CASCADE
)
TABLESPACE pg_default;


-- Index: acl_group_rules_group_id_key

-- DROP INDEX public.acl_group_rules_group_id_key;

CREATE INDEX acl_group_rules_group_id_key
    ON public.acl_group_rules USING btree
    (group_id)
    TABLESPACE pg_default;

-- Index: acl_group_rules_namespace_index

-- DROP INDEX public.acl_group_rules_namespace_index;

CREATE INDEX acl_group_rules_namespace_index
    ON public.acl_group_rules USING btree
    (namespace COLLATE pg_catalog."default")
    TABLESPACE pg_default;