// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "fang_task_state"))]
    pub struct FangTaskState;

    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "moderation_action"))]
    pub struct ModerationAction;
}

diesel::table! {
    allowed_roles (id) {
        id -> Uuid,
        guild -> Int8,
        role_id -> Int8,
        operator_role -> Int8,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::FangTaskState;

    fang_tasks (id) {
        id -> Uuid,
        metadata -> Jsonb,
        error_message -> Nullable<Text>,
        state -> FangTaskState,
        task_type -> Text,
        uniq_hash -> Nullable<Text>,
        retries -> Int4,
        scheduled_at -> Timestamptz,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    guild_settings (guild, key) {
        guild -> Int8,
        key -> Text,
        value -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ModerationAction;

    moderation_log (id) {
        id -> Uuid,
        guild -> Int8,
        kind -> ModerationAction,
        member -> Int8,
        actor -> Nullable<Int8>,
        reason -> Nullable<Text>,
        created_at -> Timestamp,
        updated_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    moderation_log_message (id) {
        id -> Int8,
        log_id -> Uuid,
        guild -> Int8,
        channel -> Int8,
    }
}

diesel::table! {
    voice_channels (id) {
        id -> Int8,
        guild -> Int8,
        creator -> Int8,
        created_at -> Timestamp,
    }
}

diesel::joinable!(moderation_log_message -> moderation_log (log_id));

diesel::allow_tables_to_appear_in_same_query!(
    allowed_roles,
    fang_tasks,
    guild_settings,
    moderation_log,
    moderation_log_message,
    voice_channels,
);
