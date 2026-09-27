# Setup

OBot is a single program: the Discord bot and the web panel run together, and store their
settings in a SQLite database.

## Requirements

- A recent stable [Rust](https://rustup.rs) toolchain (the project uses the 2024 edition)
- A Discord application with a bot
- A Hypixel API key

## Discord application

In the [Discord developer portal](https://discord.com/developers/applications), open your
application and:

1. **Bot** → copy the **Token**, it's `DISCORD_TOKEN`.
2. **Bot → Privileged Gateway Intents** → turn on **Server Members Intent**. The panel needs it
   to list server members; without it only the member table is unavailable.
3. **OAuth2** → copy the **Client Secret**, it's `DISCORD_CLIENT_SECRET`.
4. **OAuth2 → Redirects** → add `<PANEL_URL>/callback`, for example
   `http://127.0.0.1:8081/callback`. The panel's Discord login fails otherwise.

## Hypixel API key

Create one on the [Hypixel developer dashboard](https://developer.hypixel.net/dashboard), it's
`HYPIXEL_API_KEY`. OBot follows the key's rate limit on its own: it keeps a margin of 20 requests
per minute for other uses of the same key, and waits for the next minute instead of going over.

## Configuration

Copy `.env.example` to `.env` and fill it in. Variables can also come from the real environment.

### Environment files

OBot runs in a mode: `development` by default for debug builds (`cargo run`), `production` for
release builds (`cargo run --release`). Set `OBOT_ENV` in the real environment to pick another,
for example `OBOT_ENV=staging`.

Settings are read from these files, most important first, and each variable takes its value from
the first place that defines it:

1. Real environment variables, which always win
2. `.env.<mode>.local`: your machine only, for this mode
3. `.env.<mode>`: this mode, for example `.env.production`
4. `.env.local`: your machine only, every mode
5. `.env`: every mode

Missing files are skipped, so a single `.env` is enough. A mode file only needs the variables
that differ, like a test bot's token in `.env.development`. Every `.env*` file except
`.env.example` is ignored by git, since they hold secrets. On startup the bot prints its mode and
the files it read.

### Variables

| Variable                | Required | Default               | Purpose                                                        |
|-------------------------|----------|-----------------------|----------------------------------------------------------------|
| `DISCORD_TOKEN`         | yes      |                       | Bot token                                                      |
| `DISCORD_CLIENT_SECRET` | yes      |                       | OAuth2 client secret, for the panel login                      |
| `HYPIXEL_API_KEY`       | yes      |                       | Hypixel API key                                                |
| `PANEL_BIND`            | no       | `127.0.0.1:8081`      | Address the panel listens on                                   |
| `PANEL_URL`             | no       | `http://<PANEL_BIND>` | Public URL of the panel, as typed in the browser               |
| `DATABASE_URL`          | no       | `sqlite://obot.db`    | SQLite database, created on first start                        |
| `GUILD_ID`              | no       |                       | Register slash commands in this server only, which is instant. |
|                         |          |                       | Handy while developing, leave empty in production              |

`PANEL_URL` must match how people reach the panel (domain, `https`, port): it's used for the
login redirect, and panel forms posted from any other origin are rejected.

Roles, rules and nicknames aren't configured here: that's done per server in the panel.

## Running

```sh
cargo run --release
```

On startup the bot applies database migrations, logs in to Discord, and prints the panel's URL.

## Adding the bot to a server

Log in to the panel and use the **+** button in the server bar. It invites the bot with the two
permissions it needs: **Manage Roles** and **Manage Nicknames**.

Then, in the server's settings, drag the bot's role **above** every role it should give or
remove, and above the members it should rename. Discord doesn't let bots touch roles or members
above their own highest role. The panel greys out roles that are too high.

Finally, open the server in the panel and pick a verified role: `/verify` stays disabled until
then. See [Using the panel](panel.md).
