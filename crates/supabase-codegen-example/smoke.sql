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
-- These grants are for a disposable smoke database only, not a production policy.
GRANT USAGE ON SCHEMA public TO PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON public.messages TO PUBLIC;
GRANT SELECT ON public.message_summaries TO PUBLIC;
GRANT USAGE, SELECT ON SEQUENCE public.messages_id_seq TO PUBLIC;
GRANT EXECUTE ON FUNCTION public.echo_message(text) TO PUBLIC;
NOTIFY pgrst, 'reload schema';
