CREATE TABLE verified_member (
    guild_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    minecraft_uuid TEXT NOT NULL,
    minecraft_name TEXT NOT NULL,
    verified_at INTEGER NOT NULL,
    -- Admin who verified the member from the panel, NULL when they used /verify themselves
    forced_by INTEGER,
    PRIMARY KEY (guild_id, user_id)
);
