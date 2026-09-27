# Deployment

OBot has one Docker setup per environment:

| Environment | Compose file | Dockerfile | Image | Panel port | Database volume |
| --- | --- | --- | --- | --- | --- |
| Production | [`compose.production.yml`](../compose.production.yml) | [`Dockerfile.production`](../Dockerfile.production) | `ghcr.io/oblivion-fr/obot:production` | `8081` | `obot-production-data` |
| Development | [`compose.development.yml`](../compose.development.yml) | [`Dockerfile.development`](../Dockerfile.development) | `ghcr.io/oblivion-fr/obot:development` | `8082` | `obot-development-data` |

Both can run on the same host: they have their own containers, network, port and database. Use a
separate Discord application for each, so testing never touches the production bot.

- **Production** is an optimized release binary in a small image that only contains it.
- **Development** runs `cargo run` (a debug build) under
  [watchexec](https://github.com/watchexec/watchexec), which rebuilds and restarts the bot
  whenever its sources change. The first start compiles everything and takes a few minutes;
  build output is kept in a volume, so later rebuilds are incremental.

## Images

GitHub Actions ([`.github/workflows/docker.yml`](../.github/workflows/docker.yml)) builds both
images on every push to `main`, after the same checks as pre-commit (format, clippy, tests), and
publishes them to the GitHub container registry. Every image exists for x86-64 (`amd64`) and ARM64
(`arm64`, like a Raspberry Pi) under the same tag, and Docker pulls the one matching the server.
Each architecture is built on a native GitHub runner, since compiling Rust under emulation is
much slower. Each build is tagged twice:

- `production` / `development`: the latest build of `main`, used by default
- `production-<commit>` / `development-<commit>`, like `production-9bcd1a5`: that exact build,
  for pinning or rolling back with `OBOT_TAG`

Pull requests build the images without publishing them, to check they still build.

The deployed stacks only pull these images, they never build: compiling Rust on the server would
be slow, and can exceed Portainer's deployment time limit. The images follow the repository's
visibility; if the package is private, add `ghcr.io` in Portainer under **Registries** with a
GitHub token that can read packages.

## Version

The bot's Discord status shows its version and the commit it was built from, like
`OBot v0.1.0 (20c7fcf)`, followed by the environment outside production
(`OBot v0.1.0 (20c7fcf) · development`). The commit matches the image tag
`production-<commit>`. The version is the one in `Cargo.toml`: raise it for a release.

Discord doesn't allow links in a bot's status, so the clickable version is elsewhere: `/version`
replies with the commit linked to its page on GitHub and a link to the repository, and every
panel page shows the same in its footer.

## Database

The SQLite database is a file, `/data/obot.db`, in a named volume. SQLite is a library inside the
bot rather than a server, so it doesn't run in a container of its own; the volume keeps the data
apart from the bot container instead, so updating or removing the container keeps it. Back it up
by copying the volume, ideally while the stack is stopped.

## Variables

Set in Portainer (or in the shell or an env file when running Compose by hand):

| Variable                | Required | Purpose                                                                |
|-------------------------|----------|------------------------------------------------------------------------|
| `DISCORD_TOKEN`         | yes      | Bot token of this environment's Discord application                    |
| `DISCORD_CLIENT_SECRET` | yes      | Its OAuth2 client secret                                               |
| `HYPIXEL_API_KEY`       | yes      | Hypixel API key                                                        |
| `PANEL_URL`             | yes      | Public URL of the panel, like `https://obot.example.com`               |
| `GUILD_ID`              | no       | Test server for instant slash commands, usually for development only   |
| `PANEL_PORT`            | no       | Host port of the panel, `8081` in production and `8082` in development |
| `OBOT_TAG`              | no       | Image tag to run instead of the latest, like `production-9bcd1a5`      |

`OBOT_ENV`, `DATABASE_URL` and `PANEL_BIND` are set by the Compose files and the images. A
missing required variable stops the deployment with a message naming it.

Add `<PANEL_URL>/callback` to the Discord application's **OAuth2 → Redirects**. `PANEL_URL` must
match exactly how the panel is reached (scheme, domain, port): forms posted from another origin
are rejected. When the panel sits behind a reverse proxy, point the proxy to the host port and
use the proxy's public URL.

## Portainer

1. **Stacks → Add stack**, name it, and pick **Repository**.
2. Enter the repository URL and branch, and set **Compose path** to `compose.production.yml` or
   `compose.development.yml`.
3. Add the variables above under **Environment variables**.
4. **Deploy the stack**. Portainer pulls the environment's image.

To update after CI published a new image, use **Pull and redeploy** on the stack: the Compose
files always pull the image again. The database volume is kept across redeploys. To roll back,
set `OBOT_TAG` to an earlier build's tag and redeploy.

## Running Compose by hand

The variables can come from an env file:

```sh
docker compose -f compose.production.yml --env-file .env.production up -d
```

For development with hot reload, add
[`compose.development.watch.yml`](../compose.development.watch.yml) and use `watch`:

```sh
docker compose -f compose.development.yml -f compose.development.watch.yml --env-file .env.development watch
```

It builds the development image from your local sources instead of pulling it, starts the bot,
then copies every change to `src`, `templates`, `static`, `migrations` or the Cargo files into
the container, where the bot is rebuilt and restarted. Changes are copied rather than shared
through a bind mount, because Docker Desktop doesn't reliably pass file change notifications from
Windows hosts to containers.

Follow the logs with `docker compose -f compose.production.yml logs -f`. The first lines show the
environment and, once connected, the panel's URL.
