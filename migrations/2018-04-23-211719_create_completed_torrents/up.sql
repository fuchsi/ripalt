-- View: public.completed_torrents

-- DROP VIEW public.completed_torrents;

CREATE OR REPLACE VIEW public.completed_torrents AS
 SELECT tr.id,
    tr.user_id,
    tr.torrent_id,
    tr.bytes_downloaded,
    tr.bytes_uploaded,
    tr.time_seeded,
    tr.completed_at,
    t.name,
    t.size,
    COALESCE(ut.is_seeder, false) AS is_seeder,
    tl.seeder,
    tl.leecher
   FROM transfers tr
     JOIN torrents t ON t.id = tr.torrent_id
     JOIN torrent_list tl ON tl.id = tr.torrent_id
     LEFT JOIN user_transfer ut ON ut.torrent_id = tr.torrent_id AND ut.user_id = tr.user_id AND ut.is_seeder = true
  WHERE tr.completed_at IS NOT NULL;