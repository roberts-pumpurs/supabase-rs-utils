-- Run in a disposable database as its owner before starting PostgREST.
-- PostgREST must expose public with a role granted access below; no secrets are stored here.
CREATE TYPE public.mood AS ENUM ('happy', 'needs-review');
CREATE TABLE public.messages (
    id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    body text NOT NULL,
    note text,
    created_at timestamptz NOT NULL DEFAULT now(),
    mood public.mood NOT NULL DEFAULT 'happy',
    metadata jsonb NOT NULL DEFAULT '{}'::jsonb,
    amount numeric NOT NULL DEFAULT 0,
    tags text[] NOT NULL DEFAULT '{}'::text[]
);
CREATE VIEW public.message_summaries AS SELECT id, body FROM public.messages;
CREATE FUNCTION public.echo_message(message text) RETURNS text
LANGUAGE sql IMMUTABLE AS $$ SELECT message $$;
CREATE TABLE public.countries (
    id bigint PRIMARY KEY,
    name text NOT NULL
);
CREATE TABLE public.addresses (
    id bigint PRIMARY KEY,
    label text NOT NULL,
    country_id bigint,
    CONSTRAINT address_country FOREIGN KEY (country_id) REFERENCES public.countries(id)
);
CREATE TABLE public.customers (
    id bigint PRIMARY KEY,
    name text NOT NULL
);
CREATE TABLE public.orders (
    id bigint PRIMARY KEY,
    customer_id bigint NOT NULL,
    billing_id bigint,
    shipping_id bigint,
    label text NOT NULL,
    CONSTRAINT orders_customer FOREIGN KEY (customer_id) REFERENCES public.customers(id),
    CONSTRAINT orders_billing FOREIGN KEY (billing_id) REFERENCES public.addresses(id),
    CONSTRAINT orders_shipping FOREIGN KEY (shipping_id) REFERENCES public.addresses(id)
);
CREATE TABLE public.order_details (
    order_id bigint PRIMARY KEY,
    note text NOT NULL,
    CONSTRAINT details_order FOREIGN KEY (order_id) REFERENCES public.orders(id)
);
-- A UNIQUE foreign key is also a reverse to-one, without being the primary key.
CREATE TABLE public.customer_preferences (
    id bigint PRIMARY KEY,
    customer_id bigint NOT NULL UNIQUE,
    label text NOT NULL,
    CONSTRAINT preferences_customer FOREIGN KEY (customer_id) REFERENCES public.customers(id)
);
CREATE TABLE public.composite_parents (
    tenant_id bigint NOT NULL,
    id bigint NOT NULL,
    label text NOT NULL,
    PRIMARY KEY (tenant_id, id)
);
CREATE TABLE public.composite_children (
    id bigint PRIMARY KEY,
    parent_id bigint NOT NULL,
    tenant_id bigint NOT NULL,
    label text NOT NULL,
    -- Deliberately pair columns in an order different from the target PK.
    CONSTRAINT composite_parent FOREIGN KEY (parent_id, tenant_id)
        REFERENCES public.composite_parents(id, tenant_id)
);
CREATE TABLE public.skills (
    id bigint PRIMARY KEY, name text NOT NULL, owner_id bigint NOT NULL, manifest jsonb NOT NULL
);
CREATE TABLE public.adapters (
    id bigint PRIMARY KEY, name text NOT NULL, owner_id bigint NOT NULL, manifest jsonb NOT NULL
);
CREATE TABLE public.artifact_wrong_type (
    id bigint PRIMARY KEY, name text NOT NULL, owner_id text NOT NULL, manifest jsonb NOT NULL
);
CREATE TABLE public.artifact_wrong_key (
    id bigint PRIMARY KEY, name text NOT NULL, "owner.id" bigint NOT NULL, manifest jsonb NOT NULL
);
CREATE TABLE public.artifact_missing (
    id bigint PRIMARY KEY, name text NOT NULL, manifest jsonb NOT NULL
);
CREATE TABLE public.artifact_links (
    id bigint PRIMARY KEY, skill_id bigint NOT NULL, adapter_id bigint NOT NULL,
    CONSTRAINT links_skill FOREIGN KEY (skill_id) REFERENCES public.skills(id),
    CONSTRAINT links_adapter FOREIGN KEY (adapter_id) REFERENCES public.adapters(id)
);
GRANT SELECT, INSERT, UPDATE, DELETE ON public.artifact_links TO PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.skills, public.adapters,
    public.artifact_wrong_type, public.artifact_wrong_key, public.artifact_missing TO PUBLIC;
-- These grants are for a disposable smoke database only, not a production policy.
GRANT USAGE ON SCHEMA public TO PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.messages TO PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.countries, public.addresses,
    public.customers, public.orders, public.order_details, public.customer_preferences,
    public.composite_parents, public.composite_children TO PUBLIC;
GRANT SELECT ON public.message_summaries TO PUBLIC;
GRANT USAGE, SELECT ON SEQUENCE public.messages_id_seq TO PUBLIC;
GRANT EXECUTE ON FUNCTION public.echo_message(text) TO PUBLIC;
NOTIFY pgrst, 'reload schema';
