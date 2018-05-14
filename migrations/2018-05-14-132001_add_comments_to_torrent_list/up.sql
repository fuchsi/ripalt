CREATE OR REPLACE VIEW public.torrent_list
    WITH (security_barrier=false)
    AS
     SELECT t.id,
    t.info_hash,
    t.name,
    t.category_id,
    c.name AS category_name,
    t.user_id,
    u.name AS user_name,
    t.size,
    count(f.id) AS files,
    t.visible,
    t.completed,
    COALESCE(p.seeder, 0::bigint) AS seeder,
    COALESCE(p.leecher, 0::bigint) AS leecher,
    t.last_action,
    t.last_seeder,
    t.created_at,
    COALESCE(com.comments, 0::bigint) AS comments
   FROM torrents t
     LEFT JOIN users u ON u.id = t.user_id
     JOIN categories c ON c.id = t.category_id
     JOIN torrent_files f ON f.torrent_id = t.id
     LEFT JOIN ( SELECT com.torrent_id, count(com.id) as comments FROM torrent_comments com GROUP BY com.torrent_id) com ON com.torrent_id = t.id
     LEFT JOIN ( SELECT peers.torrent_id,
            count(peers.id) FILTER (WHERE peers.seeder = true) AS seeder,
            count(peers.id) FILTER (WHERE peers.seeder = false) AS leecher
           FROM peers
          GROUP BY peers.torrent_id) p ON p.torrent_id = t.id
  GROUP BY t.id, c.id, u.id, p.seeder, p.leecher, com.comments
  ORDER BY t.created_at DESC;