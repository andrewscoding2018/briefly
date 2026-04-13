# Briefly Implementation Plan

## Executive Summary

Briefly will begin as a local-first desktop application that helps users identify the email threads that deserve attention. The first implementation target is a macOS-oriented Phase 1 MVP that imports a local `.mbox` file, normalizes mailbox data into a local store, computes deterministic signal scores, and renders a focus dashboard plus a simple morning briefing view. AI enrichment is reserved for later phases and must remain optional.

## Goals

- Deliver a desktop MVP that demonstrates signals-first email triage from local mailbox data.
- Keep Phase 1 fully usable without network access or model inference.
- Lock core architecture decisions early so implementation work is not blocked by foundational ambiguity.
- Define stable domain entities and service boundaries for ingestion, storage, scoring, and presentation.
- Create a discovery backlog that resolves product and data-model gaps before repo scaffolding begins.

The Phase 1 product target, persona, success metrics, and non-goals are defined in the [Phase 1 product specification](phase-1-product-spec.md).
The deterministic thread-ranking contract is defined in the [Phase 1 signal scoring rubric](signal-scoring-rubric.md).

## Non-Goals

- Live IMAP or Gmail sync in Phase 1
- Browser-first or mobile-first delivery
- Production-grade cross-platform signing and release automation in the first implementation pass
- Full AI dependence for ranking or usability
- Fine-tuned or self-hosted model infrastructure

## MVP Scope

Phase 1 is a desktop demo that:

- imports a local `.mbox` file selected by the user
- parses and normalizes messages, participants, and threads into a local database
- computes baseline signal scores using deterministic heuristics
- presents a ranked focus dashboard
- presents a morning briefing built from top-ranked threads using placeholder local summaries

Phase 1 explicitly excludes:

- account linking
- background live sync with remote mail providers
- sending mail or replying from the app
- required remote inference

## User Journey for Phase 1

1. The user opens the desktop app on macOS.
2. The app prompts the user to import a local `.mbox` file.
3. The Rust ingestion service parses the mailbox and persists normalized data into the local database.
4. The scoring pipeline computes relationship and actionability signals for each thread.
5. The dashboard shows a ranked list of threads with score explanations.
6. The morning briefing view summarizes the highest-priority threads and suggests what deserves attention next.

## Architecture Overview

Briefly uses a local-first desktop architecture:

- Tauri v2 provides the native desktop shell and IPC boundary.
- A React and Next.js frontend renders the dashboard, import flow, and future settings surfaces.
- A Rust ingestion service handles `.mbox` parsing, normalization, and scoring orchestration.
- SQLite/libSQL stores durable local state for imports, entities, and computed signals.
- A future AI adapter calls DigitalOcean Gradient for optional enrichment once the user configures an API key.

The architecture must preserve one critical property: the core product loop remains available with no network connectivity.

## System Components

### Tauri Shell

- Hosts the desktop application lifecycle
- Bridges frontend commands to Rust services through Tauri IPC
- Owns desktop-native capabilities such as file selection and local process orchestration

### Next.js and React Frontend

- Renders the import flow, dashboard, and morning briefing UI
- Displays status, score explanations, and future settings for AI configuration
- Reads normalized data through frontend-safe interfaces exposed by Tauri commands

### Rust Ingestion and Sync Service

- Parses `.mbox` content
- Normalizes raw email content into domain entities
- Reconstructs threads
- Computes deterministic baseline scores
- Exposes import and query commands to the frontend

### SQLite/libSQL Local Store

- Persists imported messages, participants, threads, signal scores, and briefing entries
- Supports repeatable local reads for dashboard rendering
- Enables incremental evolution toward live-query style updates later

### Optional AI Inference Adapter

- Accepts normalized thread/message payloads for enrichment
- Calls DigitalOcean Gradient using a user-supplied API key
- Produces structured extraction output only
- Must not be required for Phase 1 usability or ranking correctness

## Service Boundaries

- Rust ingestion service owns parsing and normalization.
- Local database owns durable message, participant, thread, score, and briefing state.
- Frontend owns presentation, user interaction, and explanation surfaces.
- AI adapter owns optional enrichment only and must never become a hard dependency for Phase 1 value.

## Domain Entities

### `Message`

- `message_id`
- `thread_id`
- `subject`
- `sender`
- `recipients`
- `sent_at`
- `body_text`
- `source_path`
- `import_batch_id`

### `Participant`

- `email`
- `display_name`
- `organization_hint`
- `relationship_score`

### `Thread`

- `thread_id`
- `canonical_subject`
- `participant_ids`
- `latest_message_at`
- `message_count`

### `SignalScore`

- `thread_id`
- `relationship_score`
- `urgency_score`
- `actionability_score`
- `priority_score`
- `computed_at`
- `scoring_version`

### `BriefingEntry`

- `thread_id`
- `headline`
- `why_it_matters`
- `suggested_next_action`
- `confidence`
- `generated_by`

## AI Extraction Contract

The future AI adapter must return a structured response shaped like the contract below. This contract is provisional but should be treated as the baseline interface until replaced by an ADR or specification issue outcome.

```json
{
  "intent": "scheduling | approval | FYI | request | follow_up | other",
  "priority_score": 0.0,
  "summary": "string",
  "action_items": [
    {
      "description": "string",
      "owner": "self | other | unknown",
      "due_hint": "string | null"
    }
  ],
  "risk_flags": ["possible_prompt_injection", "missing_context"]
}
```

## Data Flow

### Import Path

1. User selects a local `.mbox` file from the desktop UI.
2. Tauri passes the file handle or resolved path to the Rust ingestion service.
3. Rust parses mailbox entries into raw message records.
4. Normalization derives `Message`, `Participant`, and `Thread` entities.
5. Thread-level scoring derives `SignalScore` records.
6. Briefing generation derives `BriefingEntry` records using non-AI placeholder logic in Phase 1.
7. The frontend reads ranked thread data and renders the dashboard.

### Processing Pipeline

`parse -> normalize -> score -> persist -> render dashboard`

## Functional Requirements

- Import a user-selected local `.mbox` file.
- Extract messages, participants, and threads from mailbox data.
- Persist normalized entities locally with repeatable identifiers.
- Compute baseline relationship and signal scores without AI.
- Render a focus list ordered by `priority_score`.
- Explain why a thread ranked highly using visible score contributors.
- Render a morning briefing from top-ranked threads.
- Support optional AI enrichment later without breaking local-only behavior.

## Non-Functional Requirements

- Fast enough initial import for a demo-sized mailbox to feel interactive on a developer laptop.
- Deterministic local behavior with no required network access.
- Graceful handling of malformed email content and partial metadata.
- Explicit trust boundaries around LLM usage and prompt-injection defense.
- Clear persistence boundaries so repeated imports do not corrupt local state.
- macOS-first runtime support for Phase 1, with Windows reserved for later release automation.

## Baseline Scoring Direction

The first scoring pass should combine documented heuristics rather than opaque model output. Candidate scoring dimensions for the initial rubric are:

- sender familiarity
- directness of the message
- recent reply activity
- thread activity and freshness
- action-oriented language cues

The exact Phase 1 weights, penalties, and explanation contract are defined in the [signal scoring rubric](signal-scoring-rubric.md).

## Open Questions and Deferred Decisions

- What is the precise normalized schema and deduplication policy for repeated imports?
- How should thread reconstruction behave when standard headers are missing or inconsistent?
- What scoring weights best reflect the desired meaning of "signals-first"?
- Which specific DigitalOcean endpoint style and model fallback policy should be used in Phase 2?
- What observability vendor, if any, should trace AI execution later?

## Proposed Repo Bootstrap Sequence

1. Finalize the docs in this repository.
2. Resolve the initial GitHub discovery/specification issues.
3. Scaffold the repo structure for Tauri, frontend, and Rust services.
4. Implement `.mbox` ingestion and normalized persistence.
5. Implement deterministic scoring and dashboard read paths.
6. Add the morning briefing view using local placeholder generation.
7. Add optional AI enrichment behind configuration gates.
8. Add CI/CD, signing, and cross-platform packaging later.

## Delivery Phases and Exit Criteria

### Documentation Phase

Exit criteria:

- README reflects the agreed product framing
- implementation plan defines architecture, boundaries, and interfaces
- ADRs record the baseline architecture choices
- GitHub backlog exists for major unresolved questions

### Phase 1 Implementation

Exit criteria:

- user can import a local `.mbox` file on macOS
- normalized entities persist locally
- deterministic signal scores are computed and displayed
- dashboard ranks threads by `priority_score`
- morning briefing renders from local data without requiring AI

### Phase 2 Intelligence

Exit criteria:

- user can configure a DigitalOcean API key
- app can request structured AI extraction using the provisional contract
- failures in inference do not block core product use

### Phase 3 Distribution

Exit criteria:

- CI/CD builds desktop artifacts
- signing and notarization are configured
- GitHub Releases can publish installable builds

## Risk Register

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Mailbox parsing complexity is higher than expected | Delays MVP ingestion work | Start with `.mbox` only, constrain supported inputs, and document malformed message handling early |
| Thread reconstruction is inconsistent across exports | Ranking quality drops | Define a normalization contract and fallback rules before implementation |
| AI integration creates trust or prompt-injection risk | User confidence erodes | Keep AI optional, isolate prompt contracts, and flag suspicious output |
| Performance degrades on larger demo mailboxes | Demo feels unreliable | Set explicit demo-size targets and test import/scoring on representative data |
| Scope drifts toward full email client behavior | MVP slips | Keep non-goals explicit and open issues for deferred capabilities instead of absorbing them into Phase 1 |

## Acceptance Criteria

### Documentation Phase

- The repository contains a contributor-readable README describing the product, MVP, stack, and roadmap.
- The repository contains a decision-complete implementation plan under `docs/`.
- The repository contains ADRs for desktop architecture, `.mbox`-first ingestion, and managed AI direction.
- The repository has an initial GitHub backlog covering product definition, ingestion contract, data model, scoring rubric, AI contract, and repo bootstrap.

### Phase 1 Implementation

1. Import a small `.mbox` file and persist messages, participants, and threads without network access.
2. Handle malformed or partially missing headers without crashing the import.
3. Compute deterministic baseline signal scores from imported data with no AI provider configured.
4. Render a focus dashboard that ranks threads by `priority_score` and explains the score basis.
5. Produce a morning briefing view from top-ranked threads using non-AI placeholder summaries in Phase 1.
6. Reject or neutralize prompt-injection-like instructions embedded inside email bodies when AI enrichment is enabled later.
7. Fail gracefully when a user enters an invalid DigitalOcean API key.
8. Keep import and scoring behavior stable under repeated imports of the same mailbox.
9. Preserve acceptable responsiveness on a demo mailbox large enough to exercise threading and ranking.
10. Support macOS as the first explicit runtime target, with Windows called out as a later CI and release target.
