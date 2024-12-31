CREATE TYPE channel_action AS ENUM ('update_slowmode', 'create_temp_voice');
-- we don't need other events for voice channel since there's discord's built audit log.

CREATE TABLE channel_log (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    guild BIGINT NOT NULL,
    channel BIGINT NOT NULL,
    action channel_action NOT NULL,
    actor BIGINT
);