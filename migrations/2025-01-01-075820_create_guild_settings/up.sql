CREATE TABLE guild_settings (
    guild BIGINT NOT NULL,
    key TEXT NOT NULL,
    value TEXT,
    PRIMARY KEY (guild, key)
);