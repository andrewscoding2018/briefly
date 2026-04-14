# Briefly

Briefly is a signals-first desktop email companion designed to reduce inbox overload by separating high-value human communication from low-value system noise. The Phase 1 product is a local-first macOS desktop app that imports a user's `.mbox` archive, computes baseline relationship and actionability signals, and presents a focused dashboard of threads that deserve attention.

## The Problem

Traditional inboxes flatten every message into the same list, forcing users to manually distinguish meaningful conversations from bulk notifications, receipts, and background noise. Briefly starts from the opposite premise: inbox software should elevate relationship strength and actionable intent instead of treating every message as equivalent.

## MVP

The initial MVP is a Phase 1 desktop demo with:

- Local `.mbox` import
- Local parsing, normalization, and scoring
- A focus dashboard that ranks important threads
- A morning briefing view generated from the top-ranked items

Phase 1 is intentionally useful without any cloud dependency or AI provider configuration.

## Core Principles

- Local-first UX for responsiveness and user trust
- Signals-first ranking based on relationship strength and actionable intent
- Managed AI as optional augmentation, not a requirement for core value
- Privacy-aware architecture with explicit boundaries around remote inference

## Proposed Stack

- Tauri v2 for the desktop shell
- React, Next.js, and TypeScript for the UI layer
- Rust sidecar/services for mailbox ingestion and background processing
- SQLite/libSQL for local durable state
- DigitalOcean Gradient for future managed inference

## Roadmap

### Phase 1: Local MVP

- Import `.mbox` files
- Normalize messages, participants, and threads
- Compute baseline signal scores without AI
- Render the focus dashboard and morning briefing primitives

### Phase 2: Managed Intelligence

- Add user-supplied DigitalOcean API key configuration
- Enrich messages with structured AI extraction
- Generate higher-quality summaries and action items

### Phase 3: Distribution

- Add CI/CD for macOS and Windows builds
- Configure signing and notarization
- Ship release artifacts through GitHub Releases

## Repository Guide

- [Implementation plan](docs/implementation-plan.md)
- [Repo bootstrap plan](docs/repo-bootstrap-plan.md)
- [AI extraction contract](docs/ai-extraction-contract.md)
- [Local data model](docs/local-data-model.md)
- [Phase 1 product specification](docs/phase-1-product-spec.md)
- [Signal scoring rubric](docs/signal-scoring-rubric.md)
- [Architecture decision records](docs/adr/README.md)

## Current Status

This repository has an initial bootstrap skeleton for the desktop app, Rust
workspace, contracts, and fixtures. The baseline product direction, interfaces,
and backlog are still being documented before ingestion, persistence, and
scoring logic are implemented in earnest.

## Local Setup

Use the repo-owned scripts for day-to-day development:

- `./scripts/setup` verifies the current macOS host prerequisites, prints
  Conductor-aware workspace context when available, and installs JavaScript
  dependencies with `pnpm install`.
- `./scripts/run` starts the desktop development server through the repo-level
  `pnpm dev` command.
- `./scripts/check` runs the full validation surface and is the preferred
  portable entrypoint for Docker or CI because it does not rely on a desktop GUI
  session.
- `docker compose run --rm checks` runs the same validation surface inside the
  repo's container image.

Current host prerequisites:

- Node.js 22.x
- `pnpm` 10.x
- Rust stable with `clippy` and `rustfmt`
- Xcode Command Line Tools on macOS

The setup flow intentionally verifies these tools and points to manual install
steps when they are missing instead of trying to mutate the host automatically.

## Docker

The repository includes a `Dockerfile` and `docker-compose.yml` for portable
validation. This container path is intentionally limited to `./scripts/check`,
which runs lint, build, and test. It does not try to run the Tauri desktop app
inside Docker.

Use:

- `docker compose build checks`
- `docker compose run --rm checks`

## Local Checks

Use these repo-level commands before opening or merging a PR:

- `pnpm lint` runs frontend ESLint, Rust `clippy`, and lightweight JSON validation for contract and fixture files.
- `pnpm format` applies Prettier across the repo and `cargo fmt --all` for Rust code.
- `pnpm format:check` verifies formatting without rewriting files.
- `pnpm build` runs the desktop Next.js build and `cargo build --workspace`.
- `pnpm test` runs the desktop Node test suite and `cargo test --workspace`.

Command scope is intentionally explicit:

- JavaScript and TypeScript checks currently cover the `apps/desktop` package only.
- Rust checks cover every crate in the workspace, including the Tauri shell crate.
- Docs validation is limited to JSON-backed contracts, fixtures, and app config files. Markdown linting is not enforced yet.

## PR Policy

`main` should stay protected by a lightweight required-check policy that matches
the repository's actual GitHub Actions workflow and local contributor commands.

The current branch protection policy for `main` is:

- require the stable pre-merge jobs `lint`, `build`, and `test`
- require branches to be up to date before merge
- keep reviewer requirements lightweight; external contributors should still get one approval, while owner-authored maintenance PRs can remain self-mergeable when appropriate

The pull request template reflects the local checks contributors are expected to
run before merge.
