-- Keys of the services calling the API (bot, dashboard). Only a hash of each key is stored, so
-- the database alone can't be used to call the API.
CREATE TABLE api_key (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    key_hash TEXT NOT NULL UNIQUE,
    -- Unix time in seconds
    created_at INTEGER NOT NULL,
    revoked_at INTEGER
);
