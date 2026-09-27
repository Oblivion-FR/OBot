-- Nickname texts of single values, like the rank SUPERSTAR shown as `★MVP++★`. A NULL part uses
-- the field's default, an empty one shows nothing.
CREATE TABLE nickname_value_text (
    guild_id INTEGER NOT NULL,
    field TEXT NOT NULL,
    value TEXT NOT NULL,
    prefix TEXT,
    label TEXT,
    suffix TEXT,
    PRIMARY KEY (guild_id, field, value)
);
