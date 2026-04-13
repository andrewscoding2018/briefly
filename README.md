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
- [Local data model](docs/local-data-model.md)
- [Phase 1 product specification](docs/phase-1-product-spec.md)
- [Signal scoring rubric](docs/signal-scoring-rubric.md)
- [Architecture decision records](docs/adr/README.md)

## Current Status

This repository is currently in architecture-definition mode. The baseline product direction, interfaces, and backlog are being documented before any app scaffolding or implementation work begins.
