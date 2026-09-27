-- Players without a rank get no brackets by default, like new servers: an empty prefix and
-- suffix, kept as a custom text so it can be reset. Only for servers that saved their nickname
-- format without one.
INSERT INTO nickname_value_text (guild_id, field, value, prefix, label, suffix)
SELECT DISTINCT guild_id, 'hypixel_rank', 'NO_RANK', '', NULL, ''
FROM nickname_segment
WHERE true
ON CONFLICT (guild_id, field, value) DO NOTHING;
