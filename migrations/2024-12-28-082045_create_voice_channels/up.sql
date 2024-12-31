CREATE TABLE voice_channels (
    id BIGINT PRIMARY KEY,
    guild BIGINT NOT NULL,
    creator BIGINT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT current_timestamp
);