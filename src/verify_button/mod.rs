//! A message with a Verify button, posted from the panel: clicking it asks for the Minecraft
//! username in a form, then verifies like `/verify`. The IDs are fixed, so messages posted before
//! a restart keep working.

use poise::serenity_prelude as serenity;

use crate::commands::{data_notice, verify_member};
use crate::i18n::Lang;
use crate::{Data, Error};

const BUTTON_ID: &str = "obot-verify";
const MODAL_ID: &str = "obot-verify-modal";
const USERNAME_ID: &str = "username";

/// The message members see in the channel, in the server's language
pub fn message(lang: Lang) -> serenity::CreateMessage {
    let button = serenity::CreateButton::new(BUTTON_ID)
        .label(lang.t("verify-button"))
        .style(serenity::ButtonStyle::Success);
    serenity::CreateMessage::new()
        .content(format!(
            "{}\n{}",
            lang.t("verify-message"),
            data_notice(lang)
        ))
        .components(vec![serenity::CreateActionRow::Buttons(vec![button])])
}

/// The form asking for the username, in the language of the member who clicked
fn modal(lang: Lang) -> serenity::CreateModal {
    let input = serenity::CreateInputText::new(
        serenity::InputTextStyle::Short,
        lang.t("verify-modal-username"),
        USERNAME_ID,
    )
    .placeholder("Notch")
    // Minecraft usernames are 3 to 16 characters
    .min_length(3)
    .max_length(16)
    .required(true);
    serenity::CreateModal::new(MODAL_ID, lang.t("verify-modal-title"))
        .components(vec![serenity::CreateActionRow::InputText(input)])
}

fn submitted_username(rows: &[serenity::ActionRow]) -> String {
    rows.iter()
        .flat_map(|row| &row.components)
        .find_map(|component| match component {
            serenity::ActionRowComponent::InputText(input) if input.custom_id == USERNAME_ID => {
                input.value.clone()
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// Answers clicks on the button and submissions of its form, ignores every other event
pub async fn handle(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &Data,
) -> Result<(), Error> {
    let serenity::FullEvent::InteractionCreate { interaction } = event else {
        return Ok(());
    };
    match interaction {
        serenity::Interaction::Component(click) if click.data.custom_id == BUTTON_ID => {
            let lang = Lang::from_tag(&click.locale).unwrap_or_default();
            click
                .create_response(
                    &ctx.http,
                    serenity::CreateInteractionResponse::Modal(modal(lang)),
                )
                .await?;
        }
        serenity::Interaction::Modal(submit) if submit.data.custom_id == MODAL_ID => {
            let lang = Lang::from_tag(&submit.locale).unwrap_or_default();
            let (Some(guild_id), Some(member)) = (submit.guild_id, submit.member.as_ref()) else {
                return Ok(());
            };
            // The API calls can exceed Discord's 3 second reply window
            let defer = serenity::CreateInteractionResponseMessage::new().ephemeral(true);
            submit
                .create_response(&ctx.http, serenity::CreateInteractionResponse::Defer(defer))
                .await?;
            let username = submitted_username(&submit.data.components);
            let result = verify_member(
                &ctx.http, &ctx.cache, data, guild_id, member, &username, lang,
            )
            .await;
            // A failure still answers, or the member would wait on "thinking" forever
            let reply = match &result {
                Ok(reply) => reply.clone(),
                Err(_) => lang.t("verify-failed"),
            };
            submit
                .edit_response(
                    &ctx.http,
                    serenity::EditInteractionResponse::new().content(reply),
                )
                .await?;
            result?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests;
