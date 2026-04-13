# Briefly Repo Bootstrap Plan

## Status

Proposed baseline for Issue #6.

Blocked by:

- #1 Define MVP persona, success metrics, and non-goals
- #2 Specify Phase 1 mailbox ingestion and normalization contract
- #3 Design local data model for messages, participants, threads, and signal scores
- #4 Define signal scoring rubric for relationship strength and actionable intent
- #5 Specify AI extraction contract and prompt-injection safety rules

## Purpose

This document defines the initial repository layout and bootstrap checklist for
turning Briefly's documentation into executable code. It is intentionally a
planning artifact, not a scaffolding script, because the repository should not
lock implementation details ahead of the product, ingestion, schema, and scoring
contracts that shape the code boundaries.

The plan maps directly to the current architecture:

- Tauri owns the desktop shell and IPC boundary
- a TypeScript frontend owns product UI and presentation logic
- Rust crates own ingestion, storage, scoring, and optional AI integration
- docs remain first-class source material for contracts and decisions

## Bootstrap Principles

- create only the minimum workspace boundaries needed for Phase 1
- separate source-of-truth Rust domain logic from Tauri shell glue
- keep frontend code in one app package until a second UI package is justified
- isolate schemas, fixtures, and contracts from runtime code so they can drive tests
- prefer explicit test directories and seeded fixtures over ad hoc sample files

## Proposed Top-Level Structure

```text
briefly/
├── apps/
│   └── desktop/
│       ├── src/
│       ├── src-tauri/
│       ├── public/
│       ├── tests/
│       └── package.json
├── crates/
│   ├── briefly-ingest/
│   ├── briefly-store/
│   ├── briefly-score/
│   ├── briefly-briefing/
│   ├── briefly-contracts/
│   └── briefly-ai/
├── contracts/
│   ├── ai/
│   ├── scoring/
│   └── mailbox/
├── fixtures/
│   ├── mailbox/
│   ├── scoring/
│   ├── ui/
│   └── contracts/
├── docs/
├── scripts/
├── Cargo.toml
├── package.json
├── pnpm-workspace.yaml
└── rust-toolchain.toml
```

## Directory Responsibilities

### `apps/desktop`

The desktop app should remain the only frontend app at bootstrap time.

Expected contents:

- `src/` for React, Next.js, and TypeScript UI code
- `src-tauri/` for Tauri configuration and Rust command registration
- `tests/` for frontend integration, component, and smoke tests
- `public/` for static assets used by the desktop shell

Decision:

- use a single app package first rather than a broader multi-app workspace
- only split shared UI packages later if another runtime or substantial shared component library appears

Rationale:

- the issue backlog does not yet justify package fragmentation
- one app package keeps imports, tooling, and ownership obvious during bootstrap

### `crates/briefly-ingest`

Own `.mbox` parsing, normalization, import diagnostics, and thread assignment.

Maps to:

- [Phase 1 mailbox ingestion and normalization specification](phase-1-mailbox-ingestion-spec.md)

### `crates/briefly-store`

Own SQLite access, migrations, repositories, and persistence helpers for
canonical and derived records.

Maps to:

- [Local data model](local-data-model.md)

### `crates/briefly-score`

Own deterministic participant familiarity and thread scoring logic, including
`explanation_json` generation.

Maps to:

- [Signal scoring rubric](signal-scoring-rubric.md)

### `crates/briefly-briefing`

Own morning briefing assembly from ranked threads and local placeholder summary
logic used in Phase 1.

Rationale:

- briefing generation is a separate read model and should not be coupled to the
  ranking engine
- keeping it separate leaves room for later AI-backed enrichment without
  contaminating deterministic scoring code

### `crates/briefly-contracts`

Own shared Rust types for normalized entities, scoring outputs, and app-facing
command payloads.

Scope:

- TypeScript-facing payload shapes mirrored from the canonical docs
- serialization helpers shared by Tauri and service crates
- versioned contract types that can evolve independently from persistence code

### `crates/briefly-ai`

Own optional managed inference integration and validation of the AI extraction
contract.

Phase 1 rule:

- this crate may exist as a stub during initial bootstrap, but it must not be a
  required dependency for import, scoring, or dashboard rendering

### `contracts`

Human-readable contract snapshots that are easy to compare against runtime code
and tests.

Suggested files:

- `contracts/mailbox/normalized-message.example.json`
- `contracts/scoring/signal-score.example.json`
- `contracts/ai/extraction-response.example.json`

Rule:

- examples in this directory are documentation-adjacent contract fixtures, not
  production code

### `fixtures`

Seed data for tests, demos, and future visual regression assets.

Suggested layout:

- `fixtures/mailbox/` for tiny `.mbox` samples, malformed-message samples, and
  dedup/threading cases
- `fixtures/scoring/` for normalized-thread fixtures and expected score outputs
- `fixtures/ui/` for seeded SQLite snapshots or JSON view models used by frontend
  stories and screenshot tests
- `fixtures/contracts/` for validation payloads that should match the examples in
  `contracts/`

Decision:

- keep mailbox, scoring, and UI fixtures in one top-level fixture tree so both
  Rust and TypeScript tests can share seeded artifacts

## Package and Crate Boundaries

### JavaScript and TypeScript Boundary

Bootstrap with one workspace app package:

- `apps/desktop`

Do not create additional packages for `ui`, `config`, or `utils` at the start
unless the first implementation pass exposes clear duplication. Phase 1 benefits
more from directness than from premature monorepo decomposition.

### Rust Boundary

Bootstrap with these crates:

- `briefly-ingest` for parsing and normalization
- `briefly-store` for SQLite and migrations
- `briefly-score` for deterministic ranking
- `briefly-briefing` for briefing assembly
- `briefly-contracts` for shared types
- `briefly-ai` as an optional adapter boundary

Decision:

- do not combine ingestion and scoring into one crate

Rationale:

- the docs already describe distinct lifecycles for source-of-truth normalization
  and derived scoring snapshots
- separate crates protect test boundaries and make rescoring changes less risky

Integration rule:

- `src-tauri` depends on contracts and orchestrates crate calls, but business
  logic should live in `crates/`, not inside Tauri command handlers

## Test Layout

### Frontend Tests

Place TypeScript tests under `apps/desktop/tests/` with a small split by intent:

- `apps/desktop/tests/unit/` for utility and view-model behavior
- `apps/desktop/tests/component/` for React rendering and score explanation UI
- `apps/desktop/tests/e2e/` for import-to-dashboard smoke coverage in the desktop shell

Guidance:

- keep component fixtures close to the frontend test harness, but source them
  from `fixtures/ui/` when they represent shared seeded data

### Rust Tests

Use each crate's built-in unit test support plus crate-local integration tests:

- `crates/briefly-ingest/tests/`
- `crates/briefly-store/tests/`
- `crates/briefly-score/tests/`
- `crates/briefly-briefing/tests/`
- `crates/briefly-ai/tests/`

Shared fixture rule:

- Rust integration tests should read from the top-level `fixtures/` tree rather
  than duplicating sample mailboxes per crate

### Cross-Layer Contract Tests

Reserve a small set of end-to-end contract tests for payload compatibility:

- frontend reads seeded outputs generated from Rust-owned contract fixtures
- Tauri command tests assert serialized payloads match the documented contracts
- AI adapter tests validate both success and prompt-injection rejection envelopes

## Schemas, Contracts, and Fixture Placement

Place long-lived artifacts according to their role:

- docs-first specifications stay in `docs/`
- machine-readable or example payload contracts live in `contracts/`
- reusable test and demo data lives in `fixtures/`
- runtime migrations live under `crates/briefly-store/migrations/`

Rule of thumb:

- if it defines expected shape, put it in `contracts/`
- if it exists to seed or break tests, put it in `fixtures/`
- if it explains the why, keep it in `docs/`

## Bootstrap Checklist

### Phase A: Workspace Scaffolding

- add root `package.json` and `pnpm-workspace.yaml`
- add root `Cargo.toml` workspace and `rust-toolchain.toml`
- create `apps/desktop` with Tauri v2 and the single frontend app package
- create the initial Rust crates under `crates/`
- add placeholder `contracts/`, `fixtures/`, and `scripts/` directories

Exit criteria:

- both package and cargo workspaces install successfully
- Tauri shell can build with placeholder commands wired through the contracts crate

### Phase B: Persistence and Contracts

- translate the documented local data model into migrations inside `briefly-store`
- add versioned Rust contract types in `briefly-contracts`
- add example JSON payloads under `contracts/`
- add the first seeded fixtures for mailbox import and score output

Exit criteria:

- migrations apply on a fresh local database
- contract examples can be validated against runtime types

### Phase C: Ingestion and Scoring Skeleton

- implement `.mbox` parser and normalization skeleton in `briefly-ingest`
- wire repository interfaces in `briefly-store`
- implement initial scoring pipeline shell in `briefly-score`
- return seeded ranked threads to the frontend through Tauri

Exit criteria:

- a seeded mailbox fixture can flow from import through persisted thread ranking
- frontend can render ranked placeholder data with explanation strings

### Phase D: Product Surface

- build import UI, focus dashboard, and morning briefing screens in `apps/desktop`
- add frontend integration coverage for import, ranking display, and briefing views
- add screenshot-ready seeded datasets under `fixtures/ui/`

Exit criteria:

- the app supports the documented 5-minute Phase 1 demo path on seeded local data

## Sequencing and Dependency Notes

Execution order after the blocked issues resolve:

1. lock the docs in issues #1 through #5
2. create the workspace skeleton and empty boundaries
3. codify persistence and shared contracts
4. implement ingestion against mailbox fixtures
5. implement deterministic scoring and explanation payloads
6. implement briefing assembly and frontend read paths
7. add the optional AI crate wiring only after local-first behavior works end to end

This order prevents the team from scaffolding crates and packages that later need
to be reorganized around unresolved schema or scoring decisions.

## Deferred Decisions

- whether Next.js remains the right frontend shell once Tauri scaffolding starts
- whether `briefly-briefing` should remain separate if the Phase 1 summary logic stays trivial
- whether fixture generation should later include scripted mailbox synthesis under `scripts/`
- whether frontend visual regression tooling should live in `apps/desktop/tests/` or a dedicated workspace package
