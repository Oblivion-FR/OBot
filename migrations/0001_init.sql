CREATE TABLE guild_config (
    guild_id INTEGER PRIMARY KEY,
    verified_role_id INTEGER,
    unverified_role_id INTEGER,
    hypixel_guild_id TEXT,
    hypixel_guild_name TEXT
);

CREATE TABLE role_rule (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id INTEGER NOT NULL,
    kind TEXT NOT NULL,
    value TEXT NOT NULL DEFAULT '',
    role_id INTEGER NOT NULL
);

CREATE INDEX role_rule_guild_id ON role_rule (guild_id);
