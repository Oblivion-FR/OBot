-- Language of what OBot posts in the server, a code like `fr`. NULL is English.
ALTER TABLE guild_config ADD COLUMN language TEXT;
ALTER TABLE guild_config ADD COLUMN log_channel_id INTEGER;
