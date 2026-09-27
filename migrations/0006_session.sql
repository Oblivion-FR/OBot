-- Panel logins, kept across restarts. Only a hash of the cookie's token is stored, so the
-- database alone can't be used to log in.
CREATE TABLE session (
    token_hash TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    avatar_url TEXT NOT NULL,
    lang TEXT,
    -- Unix time in seconds
    expires_at INTEGER NOT NULL
);
