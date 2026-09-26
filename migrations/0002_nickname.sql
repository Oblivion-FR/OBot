ALTER TABLE guild_config ADD COLUMN nickname_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE guild_config ADD COLUMN nickname_separator TEXT NOT NULL DEFAULT ' ';

CREATE TABLE nickname_segment (
    guild_id INTEGER NOT NULL,
    field TEXT NOT NULL,
    position INTEGER NOT NULL,
    enabled INTEGER NOT NULL,
    prefix TEXT NOT NULL,
    suffix TEXT NOT NULL,
    importance INTEGER NOT NULL,
    PRIMARY KEY (guild_id, field)
);
