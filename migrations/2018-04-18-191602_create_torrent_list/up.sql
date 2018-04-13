-- View: public.torrent_list

-- DROP VIEW public.torrent_list;

CREATE OR REPLACE VIEW public.torrent_list AS
 SELECT
    t.id,
    t.info_hash,
    t.name,
    t.category_id,
    c.name AS "category_name",
    t.user_id,
    u.name AS "user_name",
    t.size,
    COUNT(f.id) AS "files",
    t.visible,
    t.completed,
    COALESCE(p.seeder, 0) AS "seeder",
    COALESCE(p.leecher, 0) AS "leecher",
    t.last_action,
    t.last_seeder,
    t.created_at
    FROM public.torrents AS t
    LEFT JOIN public.users AS u ON u.id = t.user_id
    INNER JOIN public.categories AS c ON c.id = t.category_id
    INNER JOIN public.torrent_files AS f ON f.torrent_id = t.id
    LEFT JOIN (
        SELECT torrent_id, COUNT(id) FILTER (WHERE seeder = E'true') AS "seeder", COUNT(id) FILTER (WHERE seeder = E'false') AS "leecher"
        FROM public.peers
        GROUP BY torrent_id
    ) AS p ON p.torrent_id = t.id
    GROUP BY t.id, c.id, u.id, p.seeder, p.leecher
    ORDER BY t.created_at DESC

