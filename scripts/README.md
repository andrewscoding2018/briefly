# Scripts

Repo-owned developer entrypoints live here.

## Commands

- `./scripts/setup` verifies the host toolchain for this macOS desktop workspace,
  highlights useful Conductor environment variables when present, and installs
  JavaScript dependencies with `pnpm install`.
- `./scripts/run` starts the desktop development server through the repo-level
  `pnpm dev` command after confirming the workspace has already been bootstrapped.
- `./scripts/check` runs the repo validation surface (`pnpm lint`, `pnpm build`,
  and `pnpm test`) in one place. This is the best entrypoint for Docker or CI,
  because it does not depend on a local desktop GUI session.
- `docker compose run --rm checks` invokes that same `./scripts/check` flow
  inside the repo's container image.

## Conductor

When the scripts detect Conductor, they use these environment variables if
present:

- `CONDUCTOR_WORKSPACE_PATH`
- `CONDUCTOR_ROOT_PATH`
- `CONDUCTOR_WORKSPACE_NAME`
- `CONDUCTOR_DEFAULT_BRANCH`
- `CONDUCTOR_PORT`

The scripts still work outside Conductor by falling back to their own location
inside the repo.

## Docker

The Docker path is intentionally narrow:

- use Docker for portable validation through `./scripts/check`
- use native macOS plus Conductor for `./scripts/setup` and `./scripts/run`

The repo does not attempt to run the Tauri desktop shell from inside Docker.

## Manual macOS Prerequisites

The current scaffold still depends on host-installed tools. `./scripts/setup`
verifies them, but it does not try to mutate the machine with platform-specific
package manager logic.

- Install Node.js 22.x
- Install `pnpm` 10.x
- Install Rust stable through `rustup`, including `clippy` and `rustfmt`
- Install Xcode Command Line Tools with `xcode-select --install`
