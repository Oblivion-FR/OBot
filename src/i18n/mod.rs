//! Translations of the commands and the panel, from the Fluent files in `locales/`. They are
//! built into the binary; English is the source, every other language falls back to it.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::borrow::Cow;
use std::sync::LazyLock;

use crate::{Data, Error};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Fr,
}

/// Every `.ftl` file of a language, in `locales/<code>/`
const FILES: [(Lang, &[&str]); 2] = [
    (
        Lang::En,
        &[
            include_str!("../../locales/en/commands.ftl"),
            include_str!("../../locales/en/panel.ftl"),
        ],
    ),
    (
        Lang::Fr,
        &[
            include_str!("../../locales/fr/commands.ftl"),
            include_str!("../../locales/fr/panel.ftl"),
        ],
    ),
];

type Bundle = FluentBundle<FluentResource>;

/// One bundle per language, in `Lang::ALL` order, or why a file couldn't be read
static BUNDLES: LazyLock<Result<Vec<Bundle>, String>> = LazyLock::new(|| {
    Lang::ALL
        .into_iter()
        .map(|lang| {
            let (_, sources) = FILES
                .iter()
                .find(|(file_lang, _)| *file_lang == lang)
                .ok_or_else(|| format!("no translation files for `{}`", lang.code()))?;
            let id = lang
                .code()
                .parse()
                .map_err(|error| format!("invalid language `{}`: {error}", lang.code()))?;
            let mut bundle = Bundle::new_concurrent(vec![id]);
            // Fluent wraps variables in invisible direction marks, which would end up in
            // nicknames, URLs and copied names
            bundle.set_use_isolating(false);
            for source in *sources {
                let resource =
                    FluentResource::try_new((*source).to_owned()).map_err(|(_, errors)| {
                        format!("invalid `{}` translations: {errors:?}", lang.code())
                    })?;
                bundle.add_resource(resource).map_err(|errors| {
                    format!("duplicate `{}` translations: {errors:?}", lang.code())
                })?;
            }
            Ok(bundle)
        })
        .collect()
});

/// Fails when a translation file is invalid, so a broken file stops the bot at startup instead
/// of showing message ids
pub fn check() -> Result<(), Error> {
    BUNDLES
        .as_ref()
        .map(|_| ())
        .map_err(|error| error.clone().into())
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::En, Lang::Fr];

    pub fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
        }
    }

    /// Name of the language in itself, for the language picker
    pub fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Fr => "Français",
        }
    }

    /// From a language tag like Discord's `fr` or `en-US`, or an `Accept-Language` entry
    pub fn from_tag(tag: &str) -> Option<Self> {
        let primary = tag
            .trim()
            .split(['-', '_', ';'])
            .next()?
            .to_ascii_lowercase();
        Self::ALL.into_iter().find(|lang| lang.code() == primary)
    }

    /// Discord locales a command's texts are registered for
    fn discord_locales(self) -> &'static [&'static str] {
        match self {
            Self::En => &["en-US", "en-GB"],
            Self::Fr => &["fr"],
        }
    }

    /// A translated message
    pub fn t(self, id: &str) -> String {
        self.t_args(id, &[])
    }

    /// A translated message with one variable, for templates
    pub fn t1<'a>(self, id: &str, name: &str, value: impl Into<FluentValue<'a>>) -> String {
        self.t_args(id, &[(name, value.into())])
    }

    /// A translated message with two variables, for templates
    pub fn t2<'a>(
        self,
        id: &str,
        name1: &str,
        value1: impl Into<FluentValue<'a>>,
        name2: &str,
        value2: impl Into<FluentValue<'a>>,
    ) -> String {
        self.t_args(id, &[(name1, value1.into()), (name2, value2.into())])
    }

    /// A translated message with its variables. Falls back to English, then to the id itself.
    pub fn t_args(self, id: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let Ok(bundles) = BUNDLES.as_ref() else {
            return id.to_owned();
        };
        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, value.clone());
        }
        [self, Lang::En]
            .into_iter()
            .find_map(|lang| {
                let bundle = &bundles[lang as usize];
                let pattern = bundle.get_message(id)?.value()?;
                let mut errors = Vec::new();
                Some(
                    bundle
                        .format_pattern(pattern, Some(&fluent_args), &mut errors)
                        .into_owned(),
                )
            })
            .unwrap_or_else(|| id.to_owned())
    }
}

/// Translates with named variables: `tr!(lang, "verify-done", name = profile.name.as_str())`
macro_rules! tr {
    ($lang:expr, $id:expr $(, $name:ident = $value:expr)* $(,)?) => {
        $lang.t_args($id, &[$((stringify!($name), $value.into())),*])
    };
}
pub(crate) use tr;

/// Fills in each command's description, and its parameters' names and descriptions, in every
/// language from `command-<name>-…` messages. Command names stay the same in every language.
pub fn localize_commands(commands: &mut [poise::Command<Data, Error>]) {
    for command in commands {
        let name = command.name.clone();
        let description_id = format!("command-{name}-description");
        command.description = Some(Cow::Owned(Lang::En.t(&description_id)));
        for lang in Lang::ALL {
            for locale in lang.discord_locales() {
                command
                    .description_localizations
                    .to_mut()
                    .push((Cow::Borrowed(*locale), Cow::Owned(lang.t(&description_id))));
            }
        }
        for parameter in &mut command.parameters {
            let name_id = format!("command-{name}-{}", parameter.name);
            let description_id = format!("{name_id}-description");
            parameter.description = Some(Cow::Owned(Lang::En.t(&description_id)));
            for lang in Lang::ALL {
                for locale in lang.discord_locales() {
                    parameter
                        .name_localizations
                        .to_mut()
                        .push((Cow::Borrowed(*locale), Cow::Owned(lang.t(&name_id))));
                    parameter
                        .description_localizations
                        .to_mut()
                        .push((Cow::Borrowed(*locale), Cow::Owned(lang.t(&description_id))));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
