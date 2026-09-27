/// Discord rejects longer nicknames
pub const MAX_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Fields whose values can each get their own texts, like MVP++ shown `★MVP++★`
    pub const WITH_VALUE_TEXTS: [Field; 2] = [Self::HypixelRank, Self::GuildRankTag];

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

    /// Message id of the field's name in the panel
    pub fn label(self) -> &'static str {
        match self {
            Self::HypixelRank => "field-hypixel-rank",
            Self::Ign => "field-ign",
            Self::GuildRankTag => "field-guild-rank-tag",
            Self::GuildTag => "field-guild-tag",
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

/// Texts of one value of a field. Each part that is set replaces the field's default, so a rank
/// can keep the usual brackets with its own label, or the other way around. `Some("")` shows
/// nothing for that part.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValueText {
    pub prefix: Option<String>,
    pub label: Option<String>,
    pub suffix: Option<String>,
}

impl ValueText {
    pub fn is_default(&self) -> bool {
        self.prefix.is_none() && self.label.is_none() && self.suffix.is_none()
    }
}

#[derive(Clone)]
pub struct CustomText {
    pub field: Field,
    /// The value it applies to: a rank key like `SUPERSTAR`, or a guild rank name
    pub value: String,
    pub text: ValueText,
}

#[derive(Clone)]
pub struct NicknameFormat {
    pub enabled: bool,
    pub separator: String,
    /// In display order, one per field
    pub segments: Vec<Segment>,
    pub custom_texts: Vec<CustomText>,
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
            custom_texts: Vec::new(),
        }
    }
}

/// A value to show and the key its custom texts are found by, like the rank `SUPERSTAR` shown
/// as `MVP++`
#[derive(Clone, Copy)]
pub struct Keyed<'a> {
    pub key: &'a str,
    /// `None` shows nothing, unless the value has a custom label
    pub label: Option<&'a str>,
}

/// What a member's nickname is built from, `None` fields are skipped with their wrapping
pub struct Values<'a> {
    /// Every player has one, `NO_RANK` for players without a rank
    pub hypixel_rank: Option<Keyed<'a>>,
    pub ign: &'a str,
    /// Their rank in the linked guild, shown as its tag
    pub guild_rank: Option<Keyed<'a>>,
    pub guild_tag: Option<&'a str>,
}

impl NicknameFormat {
    pub fn needs_guild(&self) -> bool {
        self.segments
            .iter()
            .any(|segment| segment.enabled && segment.field.needs_guild())
    }

    /// Custom texts of a value; guild rank names are matched regardless of case
    pub fn custom_text(&self, field: Field, value: &str) -> Option<&ValueText> {
        self.custom_texts
            .iter()
            .find(|custom| custom.field == field && custom.value.eq_ignore_ascii_case(value))
            .map(|custom| &custom.text)
    }

    pub fn render(&self, values: &Values) -> String {
        let mut parts: Vec<(u8, String)> = self
            .segments
            .iter()
            .filter(|segment| segment.enabled)
            .filter_map(|segment| {
                let value = match segment.field {
                    Field::HypixelRank => values.hypixel_rank?,
                    Field::Ign => Keyed {
                        key: values.ign,
                        label: Some(values.ign),
                    },
                    Field::GuildRankTag => values.guild_rank?,
                    Field::GuildTag => {
                        let tag = values.guild_tag?;
                        Keyed {
                            key: tag,
                            label: Some(tag),
                        }
                    }
                };
                let custom = Field::WITH_VALUE_TEXTS
                    .contains(&segment.field)
                    .then(|| self.custom_text(segment.field, value.key))
                    .flatten();
                let prefix = custom
                    .and_then(|custom| custom.prefix.as_deref())
                    .unwrap_or(&segment.prefix);
                let label = custom
                    .and_then(|custom| custom.label.as_deref())
                    .or(value.label)?;
                let suffix = custom
                    .and_then(|custom| custom.suffix.as_deref())
                    .unwrap_or(&segment.suffix);
                (!label.is_empty())
                    .then(|| (segment.importance, format!("{prefix}{label}{suffix}")))
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
