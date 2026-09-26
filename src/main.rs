mod commands;
mod hypixel;

use poise::serenity_prelude as serenity;
use std::env::var;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    http: reqwest::Client,
    hypixel_api_key: String,
    verified_role_id: serenity::RoleId,
    unverified_role_id: serenity::RoleId,
}

fn required_env(name: &str) -> Result<String, Error> {
    var(name).map_err(|_| format!("Missing `{name}` env var, see .env.example").into())
}

fn parse_id(name: &str, value: &str) -> Result<u64, Error> {
    value
        .parse()
        .map_err(|_| format!("`{name}` must be a Discord ID, got `{value}`").into())
}

fn required_role_id(name: &str) -> Result<serenity::RoleId, Error> {
    Ok(serenity::RoleId::new(parse_id(name, &required_env(name)?)?))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // A missing .env is fine: variables can also come from the real environment
    let _ = dotenvy::dotenv();

    let token = required_env("DISCORD_TOKEN")?;
    let hypixel_api_key = required_env("HYPIXEL_API_KEY")?;
    let verified_role_id = required_role_id("VERIFIED_ROLE_ID")?;
    let unverified_role_id = required_role_id("UNVERIFIED_ROLE_ID")?;
    let dev_guild_id = var("GUILD_ID")
        .ok()
        .filter(|id| !id.is_empty())
        .map(|id| parse_id("GUILD_ID", &id).map(serenity::GuildId::new))
        .transpose()?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![commands::healthcheck(), commands::verify()],
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                let commands = &framework.options().commands;
                // Guild registration is instant, which is handy while developing
                match dev_guild_id {
                    Some(guild_id) => {
                        poise::builtins::register_in_guild(ctx, commands, guild_id).await?
                    }
                    None => poise::builtins::register_globally(ctx, commands).await?,
                }
                Ok(Data {
                    http: reqwest::Client::new(),
                    hypixel_api_key,
                    verified_role_id,
                    unverified_role_id,
                })
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();
    serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?
        .start()
        .await?;
    Ok(())
}
