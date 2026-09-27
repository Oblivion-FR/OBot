# Deployment

OBot has one Docker setup per environment:

| Environment | Compose file | Dockerfile | Panel port | Database volume |
| --- | --- | --- | --- | --- |
| Production | [`compose.production.yml`](../compose.production.yml) | [`Dockerfile.production`](../Dockerfile.production) | `8081` | `obot-production-data` |
| Development | [`compose.development.yml`](../compose.development.yml) | [`Dockerfile.development`](../Dockerfile.development) | `8082` | `obot-development-data` |

Both can run on the same host: they have their own containers, network, port and database. Use a
separate Discord application for each, so testing never touches the production bot.

- **Production** builds an optimized release binary into a small image that only contains it.
- **Development** runs `cargo run` (a debug build) under
  [watchexec](https://github.com/watchexec/watchexec), which rebuilds and restarts the bot
  whenever the code changes. The first start compiles everything and takes a few minutes; build
  output is kept in a volume, so later rebuilds are incremental.

## Database

The SQLite database is a file, `/data/obot.db`, in a named volume. SQLite is a library inside the
bot rather than a server, so it doesn't run in a container of its own; the volume keeps the data
apart from the bot container instead, so rebuilding, updating or removing the container keeps it.
Back it up by copying the volume, ideally while the stack is stopped.

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

`OBOT_ENV`, `DATABASE_URL` and `PANEL_BIND` are set by the Compose files. A missing required
variable stops the deployment with a message naming it.

Add `<PANEL_URL>/callback` to the Discord application's **OAuth2 → Redirects**. `PANEL_URL` must
match exactly how the panel is reached (scheme, domain, port): forms posted from another origin
are rejected. When the panel sits behind a reverse proxy, point the proxy to the host port and
use the proxy's public URL.

## Portainer

1. **Stacks → Add stack**, name it, and pick **Repository**.
2. Enter the repository URL and branch, and set **Compose path** to `compose.production.yml` or
   `compose.development.yml`.
3. Add the variables above under **Environment variables**.
4. **Deploy the stack**. Portainer builds the image from the environment's Dockerfile.

To update, use **Pull and redeploy** on the stack, or turn on **GitOps updates** to redeploy on
new commits. The database volume is kept across redeploys.

In Portainer the development stack runs the code of the deployed commit: redeploy it to pick up
new commits. Hot reload is for running it locally.

## Running Compose by hand

The variables can come from an env file:

```sh
docker compose -f compose.production.yml --env-file .env.production up -d --build
```

For development with hot reload, use `watch`:

```sh
docker compose -f compose.development.yml --env-file .env.development watch
```

It builds and starts the bot, then copies every change to `src`, `templates`, `static`,
`migrations` or the Cargo files into the container, where the bot is rebuilt and restarted.
Changes are copied rather than shared through a bind mount, because Docker Desktop doesn't
reliably pass file change notifications from Windows hosts to containers.

Follow the logs with `docker compose -f compose.production.yml logs -f`. The first lines show the
environment and, once connected, the panel's URL.
