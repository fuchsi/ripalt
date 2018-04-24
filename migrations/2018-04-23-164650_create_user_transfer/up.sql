-- View: public.user_transfer

-- DROP VIEW public.user_transfer;

CREATE OR REPLACE VIEW public.user_transfer AS
 SELECT p.id,
    t.id AS torrent_id,
    u.id AS user_id,
    t.name,
    p.seeder AS is_seeder,
    t.size,
    COALESCE(p2.seeder, 0::bigint) AS seeder,
    COALESCE(p2.leecher, 0::bigint) AS leecher,
    p.bytes_uploaded,
    p.bytes_downloaded,
    tr.bytes_uploaded AS total_uploaded,
    tr.bytes_downloaded AS total_downloaded
   FROM peers p
     JOIN torrents t ON t.id = p.torrent_id
     JOIN users u ON u.id = p.user_id
     JOIN transfers tr ON tr.torrent_id = t.id AND tr.user_id = u.id
     LEFT JOIN ( SELECT peers.torrent_id,
            count(peers.id) FILTER (WHERE peers.seeder = true) AS seeder,
            count(peers.id) FILTER (WHERE peers.seeder = false) AS leecher
           FROM peers
          GROUP BY peers.torrent_id) p2 ON p2.torrent_id = t.id
  GROUP BY p.id, t.id, u.id, p2.seeder, p2.leecher, tr.id;
