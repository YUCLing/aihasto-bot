CREATE TYPE moderation_action AS ENUM ('warning', 'flood', 'timeout', 'ban');

CREATE TABLE moderation_log (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    guild BIGINT NOT NULL,
    kind moderation_action NOT NULL,
    member BIGINT NOT NULL,
    actor BIGINT,
    reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
    updated_at TIMESTAMP
);