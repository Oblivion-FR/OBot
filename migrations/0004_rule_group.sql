-- Rules can be grouped under a separator role, given while any rule of the group matches
CREATE TABLE rule_group (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    guild_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    separator_role_id INTEGER NOT NULL
);

CREATE INDEX rule_group_guild_id ON rule_group (guild_id);

ALTER TABLE role_rule ADD COLUMN group_id INTEGER REFERENCES rule_group (id);
