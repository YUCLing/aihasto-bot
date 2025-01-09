CREATE TABLE moderation_log_message (
    id BIGINT PRIMARY KEY,
    log_id uuid NOT NULL REFERENCES moderation_log (id) ON DELETE CASCADE,
    guild BIGINT NOT NULL,
    channel BIGINT NOT NULL
);