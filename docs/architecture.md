# Architecture

OBot is one binary. [`main.rs`](../src/main.rs) reads the configuration from the environment and `.env` files, opens the database, then
runs the Discord bot ([poise](https://github.com/serenity-rs/poise) on
[serenity](https://github.com/serenity-rs/serenity)) and the web panel
([axum](https://github.com/tokio-rs/axum)) side by side. Both share the database, the Discord
cache and HTTP client, and the Hypixel client.

## Modules

| Module                                       | Role                                                                                      |
|----------------------------------------------|-------------------------------------------------------------------------------------------|
| [`commands`](../src/commands.rs)             | Slash commands: `/verify`, `/whois`, `/version`, `/healthcheck`                           |
| [`verification`](../src/verification/mod.rs) | Verification logic shared by `/verify` and the panel: ownership check,                    |
|                                              | role and nickname sync, removal, one account per member                                   |
| [`server_log`](../src/server_log/mod.rs)     | Posts verifications and member actions in the server's log channel                        |
| [`resync`](../src/resync/mod.rs)             | Refreshes every verified member on a schedule (`RESYNC_INTERVAL_HOURS`)                   |
| [`hypixel`](../src/hypixel/mod.rs)           | Hypixel and Mojang API types, and the shared [`Hypixel` client](../src/hypixel/client.rs) |
|                                              | with its [rate limiter](../src/hypixel/limit/mod.rs)                                      |
| [`nickname`](../src/nickname/mod.rs)         | Nickname format and rendering                                                             |
| [`version`](../src/version/mod.rs)           | Version and commit of the build (set by `build.rs`), with links to the repository         |
| [`env_files`](../src/env_files/mod.rs)       | Loads `.env` files for the current mode (`OBOT_ENV`)                                      |
| [`config`](../src/config.rs)                 | Database access: server settings, rules, nickname format, verified members                |
| [`cache`](../src/cache.rs)                   | Small time-limited cache used by the panel and the Hypixel client                         |
| [`i18n`](../src/i18n/mod.rs)                 | Translations of the panel and the commands, from the Fluent files in `locales/`           |
| [`web`](../src/web/mod.rs)                   | Panel router, shared state, access checks                                                 |
| [`web::auth`](../src/web/auth.rs)            | Discord OAuth2 login and sessions                                                         |
| [`web::pages`](../src/web/pages/mod.rs)      | Panel pages and their forms                                                               |
| [`web::members`](../src/web/members/mod.rs)  | Member table and member actions (re-verify, verify, remove)                               |

## Web panel

Pages are rendered on the server with [askama](https://github.com/askama-rs/askama) templates
from [`templates/`](../templates), which are checked at compile time. [htmx](https://htmx.org)
makes links and forms update the page without full reloads, and swaps single table rows after
member actions. [`static/app.css`](../static/app.css) and [`static/app.js`](../static/app.js)
are built into the binary.

Texts are [Fluent](https://projectfluent.org) messages from [`locales/`](../locales), one folder
per language, built into the binary. Templates translate message ids with `lang.t(...)`; the
panel language comes from the `obot_lang` cookie set by the language picker, then the Discord
account's language, then the browser's, then English. Commands reply in the language of the
member's Discord client, and their descriptions are registered in every language.

Sessions are stored in the database, by a hash of their cookie, and last a week: restarts and
updates keep everyone logged in. Access to a server is checked against Discord on every request;
only the server list in the side bar is cached, for a minute.

## Hypixel API

Every Hypixel request goes through the `Hypixel` client:

- **Rate limit**: the client reads the `RateLimit-*` headers of each response. When 20 or fewer
  requests are left in the minute, new requests wait for the next minute instead of failing. A
  request reserves its slot before it's sent, so concurrent requests can't overshoot, and a 429
  waits for the next minute and retries once.
- **Cache**: players and guilds are kept 10 minutes, but a cached answer is only used when it
  lets the action go ahead. A missing Discord link or guild membership is always fetched again,
  since the player may have just fixed it in game.

Mojang lookups (Minecraft name to account) use a separate API and aren't cached.

## Database

SQLite through [sqlx](https://github.com/launchbadge/sqlx). Migrations in
[`migrations/`](../migrations) are built into the binary and applied on startup. Discord IDs are
stored as `INTEGER`: SQLite has no unsigned integers, and every Discord ID fits in an `i64`
bit for bit.

| Table              | Content                                                                 |
|--------------------|-------------------------------------------------------------------------|
| `guild_config`     | Per server: verification roles, linked Hypixel guild, nickname settings |
| `role_rule`        | Role rules, optionally in a group                                       |
| `rule_group`       | Rule groups and their separator role                                    |
| `nickname_segment` | Nickname fields: order, prefix, suffix, importance                      |
| `verified_member`  | Which Minecraft account each member verified with, and when             |
