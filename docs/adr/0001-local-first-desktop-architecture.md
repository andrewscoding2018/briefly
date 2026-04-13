# ADR 0001: Local-First Desktop Architecture

## Status

Accepted

## Context

Briefly needs to feel fast, private, and native while avoiding early complexity from browser-based authentication flows and heavyweight desktop infrastructure. The Phase 1 product is a desktop-first MVP centered on local mailbox import and local processing.

## Decision

Adopt a Tauri v2 desktop shell with a Rust backend and a web frontend.

## Consequences

- The app can provide a lightweight native desktop footprint.
- Rust can own mailbox parsing and system-facing work close to the OS boundary.
- The frontend can use modern web tooling without defining the core runtime.
- Initial delivery targets desktop, not browser-based access.

## Rejected Alternatives

- Electron, because it carries a larger runtime footprint for an MVP whose core workload benefits from Rust-native services.
- Browser-first web app, because it increases early auth and security complexity while weakening the local-first product model.
