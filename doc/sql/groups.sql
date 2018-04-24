--
-- PostgreSQL database dump
--

-- Dumped from database version 10.3
-- Dumped by pg_dump version 10.3

-- Started on 2018-04-24 03:24:44 CEST

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET client_min_messages = warning;
SET row_security = off;

--
-- TOC entry 2293 (class 0 OID 19566)
-- Dependencies: 200
-- Data for Name: groups; Type: TABLE DATA; Schema: public; Owner: ripalt
--

INSERT INTO public.groups VALUES ('0eb8ac8f-01f4-4bf9-bb0d-e3ac0ecb15f9', 'User', NULL, NOW(), NOW());
INSERT INTO public.groups VALUES ('91c1ba93-6153-4913-9993-18ba638452d2', 'Moderator', '0eb8ac8f-01f4-4bf9-bb0d-e3ac0ecb15f9', NOW(), NOW());
INSERT INTO public.groups VALUES ('5a4517e3-f615-43f3-8852-9bb310ae688e', 'Administrator', '91c1ba93-6153-4913-9993-18ba638452d2', NOW(), NOW());
INSERT INTO public.groups VALUES ('7ad31559-5be8-40e0-9656-8b50ad1cdb39', 'Sysop', '5a4517e3-f615-43f3-8852-9bb310ae688e', NOW(), NOW());


-- Completed on 2018-04-24 03:24:44 CEST

--
-- PostgreSQL database dump complete
--

