mod cache;
mod commands;
mod config;
mod env_files;
mod hypixel;
mod i18n;
mod nickname;
mod resync;
mod verification;
mod version;
mod web;

use poise::serenity_prelude as serenity;
use std::env::var;
use std::num::NonZeroU64;
use std::sync::Arc;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    db: sqlx::SqlitePool,
    http: reqwest::Client,
    hypixel: Arc<hypixel::Hypixel>,
}

fn required_env(name: &str) -> Result<String, Error> {
    var(name).map_err(|_| format!("Missing `{name}` env var, see .env.example").into())
}

fn optional_env(name: &str) -> Option<String> {
    var(name).ok().filter(|value| !value.is_empty())
}

fn parse_id(name: &str, value: &str) -> Result<NonZeroU64, Error> {
    value
        .parse()
        .map_err(|_| format!("`{name}` must be a Discord ID, got `{value}`").into())
}

/// A rejected token would otherwise show up as a raw HTTP error dump
fn token_error(error: serenity::Error) -> Error {
    match &error {
        serenity::Error::Http(http)
            if http
                .status_code()
                .is_some_and(|status| status.as_u16() == 401) =>
        {
            "Discord rejected `DISCORD_TOKEN`: check the bot token of this environment".into()
        }
        _ => error.into(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let mode = env_files::mode(var("OBOT_ENV").ok())?;
    let loaded = env_files::load(&mode)?;
    println!("OBot v{}", version::label());
    if loaded.is_empty() {
        println!("Environment: {mode}, no .env file found, using environment variables");
    } else {
        println!("Environment: {mode}, loaded {}", loaded.join(", "));
    }

    let token = required_env("DISCORD_TOKEN")?;
    let client_secret = required_env("DISCORD_CLIENT_SECRET")?;
    let hypixel_api_key = required_env("HYPIXEL_API_KEY")?;
    let database_url =
        optional_env("DATABASE_URL").unwrap_or_else(|| "sqlite://obot.db".to_owned());
    let panel_bind = optional_env("PANEL_BIND").unwrap_or_else(|| "127.0.0.1:8081".to_owned());
    let panel_url = optional_env("PANEL_URL").unwrap_or_else(|| format!("http://{panel_bind}"));
    let dev_guild_id = optional_env("GUILD_ID")
        .map(|id| parse_id("GUILD_ID", &id).map(serenity::GuildId::from))
        .transpose()?;
    let resync_every = resync::interval(optional_env("RESYNC_INTERVAL_HOURS"))?;
    i18n::check()?;

    let db = config::connect(&database_url).await?;
    let http = reqwest::Client::new();
    let hypixel = Arc::new(hypixel::Hypixel::new(http.clone(), hypixel_api_key));

    let mut commands = vec![
        commands::healthcheck(),
        commands::version(),
        commands::verify(),
        commands::whois(),
    ];
    i18n::localize_commands(&mut commands);
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands,
            ..Default::default()
        })
        .setup({
            let (db, http, hypixel) = (db.clone(), http.clone(), hypixel.clone());
            move |ctx, ready, framework| {
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
                    Ok(Data { db, http, hypixel })
                })
            }
        })
        .build();

    // Tells at a glance which version runs, and which bot is the development one
    let status = match mode.as_str() {
        "production" => format!("OBot v{}", version::label()),
        mode => format!("OBot v{} · {mode}", version::label()),
    };
    let intents = serenity::GatewayIntents::non_privileged();
    let mut client = serenity::ClientBuilder::new(token, intents)
        .activity(serenity::ActivityData::custom(status))
        .framework(framework)
        .await?;

    // The first call to Discord, so a wrong token is reported here
    let application_id = client
        .http
        .get_current_application_info()
        .await
        .map_err(token_error)?
        .id;

    match resync_every {
        Some(every) => {
            println!(
                "Verified members are refreshed every {}h",
                every.as_secs() / 3600
            );
            let resync = resync::Resync {
                db: db.clone(),
                hypixel: hypixel.clone(),
                mojang: http.clone(),
                discord: client.http.clone(),
                cache: client.cache.clone(),
            };
            tokio::spawn(resync.run(every));
        }
        None => println!("Refreshing verified members is turned off"),
    }

    let panel = web::router(web::AppState {
        sessions: web::Sessions(db.clone()),
        db,
        cache: client.cache.clone(),
        discord: client.http.clone(),
        http_client: http,
        hypixel,
        oauth: web::OAuthConfig {
            client_id: application_id,
            client_secret,
            public_url: panel_url.trim_end_matches('/').to_owned(),
        },
        manageable_guilds: web::ManageableGuildsCache::default(),
        guild_members: web::GuildMembersCache::default(),
    });
    let listener = tokio::net::TcpListener::bind(&panel_bind).await?;
    println!("Panel listening on {panel_url}");

    tokio::select! {
        result = client.start() => result?,
        result = axum::serve(listener, panel) => result?,
    }
    Ok(())
}
