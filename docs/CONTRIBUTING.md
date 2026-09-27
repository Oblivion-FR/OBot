# Contributing

## Getting started

Follow [setup.md](setup.md) to get a bot running. A separate test Discord application
and test server are recommended: put their token, client secret and `GUILD_ID` (so slash
commands update instantly) in `.env.development`, which `cargo run` reads over `.env`. See
[environment files](setup.md#environment-files).

## Docker

The bot can also run in Docker, one stack per environment. In short:

- **Development** (`compose.development.yml`, `Dockerfile.development`): a debug build under a
  file watcher that rebuilds and restarts the bot on every change. Run it with hot reload:

  ```sh
  docker compose -f compose.development.yml --env-file .env.development watch
  ```

- **Production** (`compose.production.yml`, `Dockerfile.production`): an optimized release
  build in a small image.
- Each stack has its own panel port (`8082` for development, `8081` for production) and its own
  database volume, so both can run on the same host.
- Deployed stacks run in Portainer, which builds the image from the repository and sets the
  variables: nothing secret goes in the images or the repository.

[deployment.md](deployment.md) covers the variables, the Portainer setup and why edits are
synced into the development container instead of shared through a bind mount.

## Checks

Commits are checked by [pre-commit](https://pre-commit.com). Set it up once per clone:

```sh
pip install pre-commit
pre-commit install
```

If the `pre-commit` command isn't found after installing it, use `python -m pre_commit` instead.

On each commit:

- **Files**: trailing whitespace, final newline, valid TOML and YAML, merge conflict markers,
  large files
- **`cargo fmt --check`**, when Rust files are staged
- **`cargo clippy --all-targets -- -D warnings`**: any warning fails the commit
- **`cargo test`**

Clippy and the tests also run when templates, migrations, CSS or JS change, since they are built
into the binary. Run everything on the whole repository with `pre-commit run --all-files`.

## Commit messages

Messages follow [Conventional Commits](https://www.conventionalcommits.org), which the
`commit-msg` hook enforces:

```txt
type(optional scope): short description

Optional body.
```

Common types: `feat`, `fix`, `refactor`, `docs`, `test`, `chore`. `fixup!` commits are accepted,
for folding changes into an earlier commit with `git rebase --autosquash`.

Keep the body to what the diff doesn't show, usually why a change is needed. A header alone is
fine when there's nothing to add. For a breaking change, add a `BREAKING CHANGE:` paragraph
stating what must change on the next deploy (new variable, removed setting…).

## Code conventions

- **Tests** live next to the module they test, in a `tests.rs` file declared with
  `#[cfg(test)] mod tests;`, so they can reach private items. A module with tests is a folder:
  `foo/mod.rs` and `foo/tests.rs`.
- **Migrations** are never edited once applied anywhere: add a new numbered file in
  `migrations/`. sqlx refuses to start when an applied migration changed.
- **Templates** are checked at compile time. rust-analyzer doesn't notice changes to `.html`
  files: when it shows a template error that `cargo check` doesn't, restart it (in VS Code,
  **rust-analyzer: Restart server**).
- **Hypixel** is only called through the `Hypixel` client, so every request counts toward the
  rate limit and can use the cache.
- **Discord IDs** are stored as `INTEGER` in the database, converted with `to_db` in
  [`config.rs`](../src/config.rs).
