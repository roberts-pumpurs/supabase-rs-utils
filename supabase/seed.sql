SET session_replication_role = replica;

--
-- PostgreSQL database dump
--

-- Dumped from database version 15.1 (Ubuntu 15.1-1.pgdg20.04+1)
-- Dumped by pg_dump version 15.5 (Ubuntu 15.5-1.pgdg20.04+1)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Data for Name: audit_log_entries; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--

INSERT INTO "auth"."audit_log_entries" ("instance_id", "id", "payload", "created_at", "ip_address") VALUES
	('00000000-0000-0000-0000-000000000000', 'fa5c117c-a7d6-4b82-8da9-84553173f84a', '{"action":"user_signedup","actor_id":"00000000-0000-0000-0000-000000000000","actor_username":"service_role","actor_via_sso":false,"log_type":"team","traits":{"user_email":"supabaseutils1@yopmail.com","user_id":"874d72d1-af15-4db8-8adb-f058d1dfe5d3","user_phone":""}}', '2024-09-04 06:20:51.18272+00', ''),
	('00000000-0000-0000-0000-000000000000', 'ebef8092-e509-4a3a-83cf-f015c130a05b', '{"action":"user_signedup","actor_id":"00000000-0000-0000-0000-000000000000","actor_username":"service_role","actor_via_sso":false,"log_type":"team","traits":{"user_email":"supabaseutils2@yopmail.com","user_id":"50297aac-89c4-494f-adca-94753bd6b462","user_phone":""}}', '2024-09-04 06:21:04.711826+00', '');


--
-- Data for Name: flow_state; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: users; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--

INSERT INTO "auth"."users" ("instance_id", "id", "aud", "role", "email", "encrypted_password", "email_confirmed_at", "invited_at", "confirmation_token", "confirmation_sent_at", "recovery_token", "recovery_sent_at", "email_change_token_new", "email_change", "email_change_sent_at", "last_sign_in_at", "raw_app_meta_data", "raw_user_meta_data", "is_super_admin", "created_at", "updated_at", "phone", "phone_confirmed_at", "phone_change", "phone_change_token", "phone_change_sent_at", "email_change_token_current", "email_change_confirm_status", "banned_until", "reauthentication_token", "reauthentication_sent_at", "is_sso_user", "deleted_at") VALUES
	('00000000-0000-0000-0000-000000000000', '874d72d1-af15-4db8-8adb-f058d1dfe5d3', 'authenticated', 'authenticated', 'supabaseutils1@yopmail.com', '$2a$10$tAms0SS26/CTf.ycOzRY8uIQ9E7lhQHcMf2VkeZimKTLYofN44OQ2', '2024-09-04 06:20:51.187523+00', NULL, '', NULL, '', NULL, '', '', NULL, NULL, '{"provider": "email", "providers": ["email"]}', '{}', NULL, '2024-09-04 06:20:51.173049+00', '2024-09-04 06:20:51.187645+00', NULL, NULL, '', '', NULL, '', 0, NULL, '', NULL, false, NULL),
	('00000000-0000-0000-0000-000000000000', '50297aac-89c4-494f-adca-94753bd6b462', 'authenticated', 'authenticated', 'supabaseutils2@yopmail.com', '$2a$10$G6uw9RL5u5jTmCE7jVA1y.P3v5mVZut6GSmHFPzbf7/98JwpLtTZq', '2024-09-04 06:21:04.712793+00', NULL, '', NULL, '', NULL, '', '', NULL, NULL, '{"provider": "email", "providers": ["email"]}', '{}', NULL, '2024-09-04 06:21:04.706628+00', '2024-09-04 06:21:04.713063+00', NULL, NULL, '', '', NULL, '', 0, NULL, '', NULL, false, NULL);


--
-- Data for Name: identities; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--

INSERT INTO "auth"."identities" ("provider_id", "user_id", "identity_data", "provider", "last_sign_in_at", "created_at", "updated_at", "id") VALUES
	('874d72d1-af15-4db8-8adb-f058d1dfe5d3', '874d72d1-af15-4db8-8adb-f058d1dfe5d3', '{"sub": "874d72d1-af15-4db8-8adb-f058d1dfe5d3", "email": "supabaseutils1@yopmail.com", "email_verified": false, "phone_verified": false}', 'email', '2024-09-04 06:20:51.18099+00', '2024-09-04 06:20:51.181049+00', '2024-09-04 06:20:51.181049+00', '23f7b163-8690-44e5-91cb-02571c01a7f9'),
	('50297aac-89c4-494f-adca-94753bd6b462', '50297aac-89c4-494f-adca-94753bd6b462', '{"sub": "50297aac-89c4-494f-adca-94753bd6b462", "email": "supabaseutils2@yopmail.com", "email_verified": false, "phone_verified": false}', 'email', '2024-09-04 06:21:04.709883+00', '2024-09-04 06:21:04.709966+00', '2024-09-04 06:21:04.709966+00', 'd87f1a2a-d36a-4137-bdee-b1d1224c2cd7');


--
-- Data for Name: instances; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: sessions; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: mfa_amr_claims; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: mfa_factors; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: mfa_challenges; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: refresh_tokens; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: sso_providers; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: saml_providers; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: saml_relay_states; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: sso_domains; Type: TABLE DATA; Schema: auth; Owner: supabase_auth_admin
--



--
-- Data for Name: key; Type: TABLE DATA; Schema: pgsodium; Owner: supabase_admin
--



--
-- Data for Name: profiles; Type: TABLE DATA; Schema: public; Owner: postgres
--

INSERT INTO "public"."profiles" ("id", "first_name", "last_name") VALUES
	('874d72d1-af15-4db8-8adb-f058d1dfe5d3', NULL, NULL),
	('50297aac-89c4-494f-adca-94753bd6b462', NULL, NULL);


--
-- Data for Name: buckets; Type: TABLE DATA; Schema: storage; Owner: supabase_storage_admin
--



--
-- Data for Name: objects; Type: TABLE DATA; Schema: storage; Owner: supabase_storage_admin
--



--
-- Data for Name: hooks; Type: TABLE DATA; Schema: supabase_functions; Owner: supabase_functions_admin
--



--
-- Data for Name: secrets; Type: TABLE DATA; Schema: vault; Owner: supabase_admin
--



--
-- Name: refresh_tokens_id_seq; Type: SEQUENCE SET; Schema: auth; Owner: supabase_auth_admin
--

SELECT pg_catalog.setval('"auth"."refresh_tokens_id_seq"', 1, false);


--
-- Name: key_key_id_seq; Type: SEQUENCE SET; Schema: pgsodium; Owner: supabase_admin
--

SELECT pg_catalog.setval('"pgsodium"."key_key_id_seq"', 1, false);


--
-- Name: hooks_id_seq; Type: SEQUENCE SET; Schema: supabase_functions; Owner: supabase_functions_admin
--

SELECT pg_catalog.setval('"supabase_functions"."hooks_id_seq"', 1, false);


--
-- PostgreSQL database dump complete
--

RESET ALL;
