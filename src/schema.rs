table! {
    use diesel::sql_types::*;
    use models::acl::PermissionMapping;
    acl_group_rules (id) {
        id -> Uuid,
        namespace -> Varchar,
        group_id -> Uuid,
        permission -> PermissionMapping,
    }
}

table! {
    use diesel::sql_types::*;
    use models::acl::PermissionMapping;
    acl_user_rules (id) {
        id -> Uuid,
        namespace -> Varchar,
        user_id -> Uuid,
        permission -> PermissionMapping,
    }
}

table! {
    categories (id) {
        id -> Uuid,
        name -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    chat_messages (id) {
        id -> Uuid,
        user_id -> Uuid,
        chat -> Int2,
        message -> Text,
        created_at -> Timestamptz,
    }
}

table! {
    completed_torrents (id) {
        id -> Uuid,
        user_id -> Uuid,
        torrent_id -> Uuid,
        bytes_downloaded -> Int8,
        bytes_uploaded -> Int8,
        time_seeded -> Int4,
        completed_at -> Timestamptz,
        name -> Varchar,
        size -> Int8,
        is_seeder -> Bool,
        seeder -> Int8,
        leecher -> Int8,
    }
}

table! {
    groups (id) {
        id -> Uuid,
        name -> Varchar,
        parent_id -> Nullable<Uuid>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    message_folders (id) {
        id -> Uuid,
        user_id -> Uuid,
        name -> Varchar,
        purge -> Int2,
    }
}

table! {
    messages (id) {
        id -> Uuid,
        folder_id -> Uuid,
        sender_id -> Nullable<Uuid>,
        receiver_id -> Uuid,
        subject -> Varchar,
        body -> Text,
        is_read -> Bool,
        created_at -> Timestamptz,
    }
}

table! {
    peers (id) {
        id -> Uuid,
        torrent_id -> Uuid,
        user_id -> Uuid,
        ip_address -> Inet,
        port -> Int4,
        bytes_uploaded -> Int8,
        bytes_downloaded -> Int8,
        bytes_left -> Int8,
        seeder -> Bool,
        peer_id -> Bytea,
        user_agent -> Varchar,
        crypto_enabled -> Bool,
        crypto_port -> Nullable<Int4>,
        offset_uploaded -> Int8,
        offset_downloaded -> Int8,
        created_at -> Timestamptz,
        finished_at -> Nullable<Timestamptz>,
        updated_at -> Timestamptz,
    }
}

table! {
    torrent_files (id) {
        id -> Uuid,
        torrent_id -> Uuid,
        file_name -> Varchar,
        size -> Int8,
    }
}

table! {
    torrent_images (id) {
        id -> Uuid,
        torrent_id -> Uuid,
        file_name -> Varchar,
        index -> Int2,
        created_at -> Timestamptz,
    }
}

table! {
    torrent_meta_files (id) {
        id -> Uuid,
        data -> Bytea,
    }
}

table! {
    torrent_nfos (id) {
        id -> Uuid,
        torrent_id -> Uuid,
        data -> Bytea,
    }
}

table! {
    torrent_list (id) {
        id -> Uuid,
        info_hash -> Bytea,
        name -> Varchar,
        category_id -> Uuid,
        category_name -> Varchar,
        user_id -> Nullable<Uuid>,
        user_name -> Nullable<Varchar>,
        size -> Int8,
        files -> Int8,
        visible -> Bool,
        completed -> Int4,
        seeder -> Int8,
        leecher -> Int8,
        last_action -> Nullable<Timestamptz>,
        last_seeder -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

table! {
    torrents (id) {
        id -> Uuid,
        name -> Varchar,
        info_hash -> Bytea,
        category_id -> Uuid,
        user_id -> Nullable<Uuid>,
        description -> Text,
        size -> Int8,
        visible -> Bool,
        completed -> Int4,
        last_action -> Nullable<Timestamptz>,
        last_seeder -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    transfers (id) {
        id -> Uuid,
        user_id -> Uuid,
        torrent_id -> Uuid,
        bytes_uploaded -> Int8,
        bytes_downloaded -> Int8,
        time_seeded -> Int4,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        completed_at -> Nullable<Timestamptz>,
    }
}

table! {
    user_properties (id) {
        id -> Uuid,
        user_id -> Uuid,
        name -> Varchar,
        value -> Jsonb,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

table! {
    user_transfer (id) {
        id -> Uuid,
        torrent_id -> Uuid,
        user_id -> Uuid,
        name -> Varchar,
        is_seeder -> Bool,
        size -> Int8,
        seeder -> Int8,
        leecher -> Int8,
        bytes_uploaded -> Int8,
        bytes_downloaded -> Int8,
        total_uploaded -> Int8,
        total_downloaded -> Int8,
    }
}

table! {
    users (id) {
        id -> Uuid,
        name -> Varchar,
        email -> Varchar,
        password -> Bytea,
        salt -> Bytea,
        status -> Int2,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
        passcode -> Bytea,
        uploaded -> Int8,
        downloaded -> Int8,
        group_id -> Uuid,
        ip_address -> Nullable<Inet>,
        last_active -> Nullable<Timestamptz>,
    }
}

joinable!(acl_group_rules -> groups (group_id));
joinable!(acl_user_rules -> users (user_id));
joinable!(chat_messages -> users (user_id));
joinable!(message_folders -> users (user_id));
joinable!(messages -> message_folders (folder_id));
joinable!(peers -> torrents (torrent_id));
joinable!(peers -> users (user_id));
joinable!(torrent_images -> torrents (torrent_id));
joinable!(torrents -> categories (category_id));
joinable!(torrents -> users (user_id));
joinable!(transfers -> torrents (torrent_id));
joinable!(transfers -> users (user_id));
joinable!(user_transfer -> users (user_id));
joinable!(user_transfer -> torrents (torrent_id));
joinable!(user_properties -> users (user_id));
joinable!(users -> groups (group_id));

allow_tables_to_appear_in_same_query!(
    acl_group_rules,
    acl_user_rules,
    categories,
    chat_messages,
    groups,
    message_folders,
    messages,
    peers,
    torrent_files,
    torrent_images,
    torrent_meta_files,
    torrent_nfos,
    torrents,
    transfers,
    user_transfer,
    user_properties,
    users,
);
