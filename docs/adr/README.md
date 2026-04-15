# Architecture Decision Records

This directory records the high-level architectural decisions that define Briefly's initial direction. Each ADR uses a simple template with `Status`, `Context`, `Decision`, `Consequences`, and `Rejected Alternatives`.

Current ADRs:

- [0001: Local-first desktop architecture](0001-local-first-desktop-architecture.md)
- [0002: Phase 1 `.mbox` ingestion before live sync](0002-phase-1-mbox-ingestion-before-live-sync.md)
- [0003: Managed AI via DigitalOcean Gradient](0003-managed-ai-via-digitalocean-gradient.md)

Linking convention:

- Link the relevant ADR from `docs/implementation-plan.md` whenever a workstream depends on a documented tradeoff.
- Open or update an ADR when implementation changes a durable architecture, schema, or product-boundary decision.
- Keep GitHub issues focused on execution scope; use ADRs to explain why a non-obvious choice was made.
