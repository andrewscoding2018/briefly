# ADR 0002: Phase 1 `.mbox` Ingestion Before Live Sync

## Status

Accepted

## Context

Briefly needs a fast path to a credible demo that proves the signals-first ranking model. Live sync through IMAP or Gmail APIs would introduce account linking, auth flows, provider-specific edge cases, and broader security scope before the core ranking experience is validated.

## Decision

Phase 1 will use local `.mbox` import rather than live IMAP or Gmail sync.

## Consequences

- The MVP can be demonstrated with deterministic local datasets.
- Import, normalization, and scoring can be tested without network dependencies.
- The product avoids early OAuth and provider-integration overhead.
- Live sync remains a later-stage capability rather than a launch blocker.

## Rejected Alternatives

- IMAP-first MVP, because it expands surface area before the basic value proposition is proven.
- Gmail API-first MVP, because it introduces external platform dependencies and security requirements too early.
