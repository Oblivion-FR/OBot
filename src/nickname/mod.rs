/// Discord rejects longer nicknames
pub const MAX_LEN: usize = 32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Field {
    HypixelRank,
    Ign,
    GuildRankTag,
    GuildTag,
}

impl Field {
    pub const ALL: [Field; 4] = [
        Self::HypixelRank,
        Self::Ign,
        Self::GuildRankTag,
        Self::GuildTag,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HypixelRank => "hypixel_rank",
            Self::Ign => "ign",
            Self::GuildRankTag => "guild_rank_tag",
            Self::GuildTag => "guild_tag",
        }
    }

    pub fn parse(field: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|candidate| candidate.as_str() == field)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::HypixelRank => "Hypixel rank",
            Self::Ign => "Minecraft name",
            Self::GuildRankTag => "Guild rank tag",
            Self::GuildTag => "Guild tag",
        }
    }

    pub fn needs_guild(self) -> bool {
        matches!(self, Self::GuildRankTag | Self::GuildTag)
    }
}

#[derive(Clone)]
pub struct Segment {
    pub field: Field,
    pub enabled: bool,
    /// Wrapping characters, like `[` and `]`
    pub prefix: String,
    pub suffix: String,
    /// 1 is the most important, higher values are dropped first when the nickname is too long
    pub importance: u8,
}

#[derive(Clone)]
pub struct NicknameFormat {
    pub enabled: bool,
    pub separator: String,
    /// In display order, one per field
    pub segments: Vec<Segment>,
}

impl Default for NicknameFormat {
    /// `[<Hypixel rank>] <IGN> [<Guild rank tag>]`, disabled until an admin turns it on
    fn default() -> Self {
        let segment = |field, enabled, prefix: &str, suffix: &str, importance| Segment {
            field,
            enabled,
            prefix: prefix.to_owned(),
            suffix: suffix.to_owned(),
            importance,
        };
        Self {
            enabled: false,
            separator: " ".to_owned(),
            segments: vec![
                segment(Field::HypixelRank, true, "[", "]", 2),
                segment(Field::Ign, true, "", "", 1),
                segment(Field::GuildRankTag, true, "[", "]", 3),
                segment(Field::GuildTag, false, "[", "]", 4),
            ],
        }
    }
}

/// What a member's nickname is built from, `None` fields are skipped with their wrapping
pub struct Values<'a> {
    pub hypixel_rank: Option<&'a str>,
    pub ign: &'a str,
    pub guild_rank_tag: Option<&'a str>,
    pub guild_tag: Option<&'a str>,
}

impl NicknameFormat {
    pub fn needs_guild(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.enabled && segment.field.needs_guild())
    }

    pub fn render(&self, values: &Values) -> String {
        let mut parts: Vec<(u8, String)> = self
            .segments
            .iter()
            .filter(|segment| segment.enabled)
            .filter_map(|segment| {
                let value = match segment.field {
                    Field::HypixelRank => values.hypixel_rank,
                    Field::Ign => Some(values.ign),
                    Field::GuildRankTag => values.guild_rank_tag,
                    Field::GuildTag => values.guild_tag,
                }?;
                (!value.is_empty()).then(|| {
                    let text = format!("{}{value}{}", segment.prefix, segment.suffix);
                    (segment.importance, text)
                })
            })
            .collect();

        loop {
            let texts: Vec<&str> = parts.iter().map(|(_, text)| text.as_str()).collect();
            let nickname = texts.join(&self.separator);
            if nickname.chars().count() <= MAX_LEN || parts.len() <= 1 {
                return nickname.chars().take(MAX_LEN).collect();
            }
            // On ties, the rightmost part goes first
            let least_important = parts
                .iter()
                .enumerate()
                .max_by_key(|(index, (importance, _))| (*importance, *index))
                .map(|(index, _)| index);
            if let Some(index) = least_important {
                parts.remove(index);
            }
        }
    }
}

#[cfg(test)]
mod tests;
