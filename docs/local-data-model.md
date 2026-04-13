# Briefly Local Data Model

## Status

Proposed baseline for Issue #3

## Purpose

This document defines the Phase 1 local persistence model for imports, normalized mail entities, scoring outputs, and briefing snapshots. It is intentionally schema-oriented so the repo can bootstrap SQLite-backed storage and service boundaries without re-deciding core identifiers or update rules.

The model is designed to satisfy four constraints:

- repeated imports must be idempotent
- dashboard reads must stay simple and fast on a local SQLite store
- rescoring must not overwrite source-of-truth mail records
- future AI enrichment must attach to normalized thread data without changing the ingestion model

## Design Principles

### Source-of-Truth vs Derived Data

Source-of-truth records are immutable or slowly changing records created by import and normalization:

- `import_batches`
- `message_sources`
- `messages`
- `participants`
- `message_participants`
- `threads`

Derived records are recomputable outputs produced from normalized source-of-truth records:

- `participant_relationship_scores`
- `scoring_runs`
- `signal_scores`
- `briefing_entries`

The rule is simple: import and normalization create canonical mail entities, while scoring and briefing generation only append or replace derived snapshots.

### Stable Identifiers

Every top-level entity uses an opaque local ID so the storage layer does not depend on any external provider format:

- `import_batch_id`
- `source_record_id`
- `message_id`
- `participant_id`
- `thread_id`
- `scoring_run_id`
- `briefing_entry_id`

Opaque IDs should be generated as ULIDs or UUIDv7 values.

Canonical deduplication keys are stored separately from the primary key:

- `messages.canonical_message_key`
- `participants.normalized_email`
- `threads.thread_key`

### Performance Posture

This issue is documentation-only, so it does not change runtime behavior. The proposed model is also intentionally performance-safe for Phase 1:

- import deduplication is handled by unique canonical keys instead of expensive full-table comparisons
- dashboard reads depend on current-score snapshots, not on-demand recomputation
- briefing reads use persisted snapshot rows, not live generation per render
- re-imports append import metadata while reusing canonical message records

## Entity Model

### `import_batches`

One row per user import attempt.

| Field | Type | Notes |
| --- | --- | --- |
| `import_batch_id` | text pk | Opaque local ID |
| `source_path` | text | Original user-selected file path at import time |
| `source_filename` | text | Convenience display value |
| `source_sha256` | text | File fingerprint for repeatability |
| `file_size_bytes` | integer | Import diagnostics |
| `parser_version` | text | Parser/normalizer version used |
| `status` | text | `running`, `completed`, `partial`, `failed` |
| `started_at` | datetime | Import start |
| `completed_at` | datetime nullable | Import completion |
| `message_count_seen` | integer | Raw messages encountered |
| `message_count_linked` | integer | Messages linked to canonical rows |
| `parse_error_count` | integer | Diagnostics |
| `notes` | text nullable | Human-readable import error summary |

Indexes:

- index on `source_sha256`
- index on `started_at desc`

Rationale:

- the batch row is the audit trail for repeatability and debugging
- file fingerprinting lets the app detect duplicate imports early without relying on file path alone, without preventing explicit re-imports

### `message_sources`

Immutable import-level source records. This table preserves what was seen in a specific import batch even when the canonical message already exists.

| Field | Type | Notes |
| --- | --- | --- |
| `source_record_id` | text pk | Opaque local ID |
| `import_batch_id` | text fk | References `import_batches` |
| `message_id` | text fk nullable | References canonical `messages` row after normalization |
| `mailbox_path` | text nullable | Folder/mailbox path within the import |
| `source_position` | integer nullable | Stable mailbox order when available |
| `raw_message_sha256` | text | Raw record fingerprint |
| `header_blob` | text nullable | Serialized parsed headers for debugging |
| `parse_status` | text | `parsed`, `partial`, `failed` |
| `parse_error` | text nullable | Parse failure details |
| `created_at` | datetime | Ingest timestamp |

Indexes:

- index on `import_batch_id, source_position`
- index on `raw_message_sha256`
- index on `message_id`

Rationale:

- preserves import provenance without duplicating the canonical message body
- separates parser diagnostics from the normalized mail model

### `participants`

Canonical participant identities. This table stores stable identity attributes only, not time-varying scores.

| Field | Type | Notes |
| --- | --- | --- |
| `participant_id` | text pk | Opaque local ID |
| `normalized_email` | text unique | Lowercased canonical address |
| `display_name` | text nullable | Most recent best-known display name |
| `organization_hint` | text nullable | Derived domain/company hint |
| `first_seen_at` | datetime | Earliest linked message time |
| `last_seen_at` | datetime | Latest linked message time |
| `created_at` | datetime | Row creation |
| `updated_at` | datetime | Metadata refresh timestamp |

Indexes:

- unique index on `normalized_email`
- index on `last_seen_at desc`

Decision:

- do not store `relationship_score` directly on `participants`
- store relationship scoring in a derived table so identity and scoring lifecycles stay separate

### `messages`

Canonical message records deduplicated across imports.

| Field | Type | Notes |
| --- | --- | --- |
| `message_id` | text pk | Opaque local ID |
| `canonical_message_key` | text unique | Deterministic dedupe key |
| `internet_message_id` | text nullable | RFC `Message-ID` when available |
| `thread_id` | text fk | References `threads` |
| `subject` | text nullable | Raw subject |
| `normalized_subject` | text nullable | Subject stripped of reply prefixes |
| `sent_at` | datetime nullable | Message timestamp |
| `sender_participant_id` | text fk nullable | References `participants` |
| `body_text` | text nullable | Plaintext content |
| `body_preview` | text nullable | Short UI preview |
| `has_attachments` | boolean | Attachment hint |
| `import_first_seen_batch_id` | text fk | First batch that created this canonical row |
| `created_at` | datetime | Canonical row creation |
| `updated_at` | datetime | Last normalization update |

Indexes:

- unique index on `canonical_message_key`
- index on `thread_id, sent_at desc`
- index on `sender_participant_id`
- index on `internet_message_id`

Canonical key rules:

1. Prefer normalized RFC `Message-ID` when present.
2. Fallback to a deterministic hash of sender address, normalized subject, sent timestamp, and normalized body digest.
3. The fallback algorithm version must be tracked in the normalizer code so future changes can be migrated deliberately.

### `message_participants`

Join table for recipients and other per-message participant roles.

| Field | Type | Notes |
| --- | --- | --- |
| `message_id` | text fk | References `messages` |
| `participant_id` | text fk | References `participants` |
| `role` | text | `to`, `cc`, `bcc`, `reply_to` |
| `position` | integer | Preserves original recipient order |

Primary key:

- (`message_id`, `participant_id`, `role`)

Indexes:

- index on `participant_id, role`

### `threads`

Canonical thread containers used by the dashboard, scoring, and briefing systems.

| Field | Type | Notes |
| --- | --- | --- |
| `thread_id` | text pk | Opaque local ID |
| `thread_key` | text unique | Deterministic threading key |
| `canonical_subject` | text nullable | Stable display subject |
| `latest_message_at` | datetime nullable | Cached read optimization |
| `message_count` | integer | Cached read optimization |
| `participant_count` | integer | Cached read optimization |
| `threading_version` | text | Version of threading logic |
| `created_at` | datetime | Row creation |
| `updated_at` | datetime | Cache refresh timestamp |

Indexes:

- unique index on `thread_key`
- index on `latest_message_at desc`

Thread key rules:

1. Prefer header-based threading using `In-Reply-To` and `References` when available.
2. Fallback to a deterministic subject-plus-participant heuristic when headers are missing.
3. Normal rescoring must not rewrite `thread_id` values.
4. Re-threading is a separate migration event tied to a `threading_version` change, not part of ordinary imports.

### `participant_relationship_scores`

Derived participant-level relationship metrics computed from message history.

| Field | Type | Notes |
| --- | --- | --- |
| `participant_id` | text fk | References `participants` |
| `scoring_run_id` | text fk | References `scoring_runs` |
| `relationship_score` | real | Normalized value |
| `interaction_count` | integer | Supporting explanation data |
| `last_interaction_at` | datetime nullable | Supporting explanation data |
| `is_current` | boolean | Latest run marker |
| `computed_at` | datetime | Snapshot time |

Primary key:

- (`participant_id`, `scoring_run_id`)

Indexes:

- index on `participant_id, is_current`
- index on `scoring_run_id`

Decision:

- relationship score is stored as a derived snapshot, not on the participant identity row

### `scoring_runs`

One row per scoring execution over the canonical dataset.

| Field | Type | Notes |
| --- | --- | --- |
| `scoring_run_id` | text pk | Opaque local ID |
| `trigger_import_batch_id` | text fk nullable | Import that initiated the run |
| `scoring_version` | text | Heuristic/scoring rules version |
| `status` | text | `running`, `completed`, `failed` |
| `started_at` | datetime | Run start |
| `completed_at` | datetime nullable | Run finish |
| `notes` | text nullable | Error or operator notes |

Indexes:

- index on `started_at desc`
- index on `scoring_version`

Rationale:

- makes rescoring explicit and auditable
- allows current derived rows to be replaced atomically per run

### `signal_scores`

Derived thread-level scoring outputs used by the dashboard.

| Field | Type | Notes |
| --- | --- | --- |
| `thread_id` | text fk | References `threads` |
| `scoring_run_id` | text fk | References `scoring_runs` |
| `relationship_score` | real | Thread-level aggregate |
| `urgency_score` | real | Deterministic heuristic |
| `actionability_score` | real | Deterministic heuristic |
| `priority_score` | real | Dashboard sort key |
| `explanation_json` | text nullable | Machine-readable explanation payload defined by the scoring rubric |
| `is_current` | boolean | Latest run marker |
| `computed_at` | datetime | Snapshot time |

Primary key:

- (`thread_id`, `scoring_run_id`)

Indexes:

- index on `thread_id, is_current`
- index on `is_current, priority_score desc`
- index on `scoring_run_id`

Decision:

- keep one historical row per scoring run so ranking changes are inspectable
- the UI should read only rows where `is_current = true`

### `briefing_entries`

Persisted morning briefing snapshots generated from current thread scores.

| Field | Type | Notes |
| --- | --- | --- |
| `briefing_entry_id` | text pk | Opaque local ID |
| `scoring_run_id` | text fk | References `scoring_runs` |
| `thread_id` | text fk | References `threads` |
| `headline` | text | User-facing summary line |
| `why_it_matters` | text | Reason for inclusion |
| `suggested_next_action` | text nullable | Recommended next step |
| `confidence` | real | Placeholder confidence value |
| `generated_by` | text | `local_rules` in Phase 1 |
| `rank_position` | integer | Display order |
| `is_current` | boolean | Latest briefing marker |
| `generated_at` | datetime | Snapshot time |

Indexes:

- index on `is_current, rank_position`
- index on `thread_id`
- index on `scoring_run_id`

Decision:

- briefing entries persist as snapshots per scoring run
- they should not regenerate on every read in Phase 1 because persisted snapshots produce stable demos and faster UI loads

## Relationships

The normalized model centers on canonical messages and threads:

- one `import_batch` has many `message_sources`
- one `message_source` resolves to zero or one canonical `message`
- one `thread` has many `messages`
- one `message` has one sender and many recipient links
- one `participant` can appear across many messages and threads
- one `scoring_run` has many `participant_relationship_scores`
- one `scoring_run` has many `signal_scores`
- one `scoring_run` has many `briefing_entries`

Operationally:

- `messages`, `participants`, and `threads` are the stable normalized graph
- `signal_scores` and `briefing_entries` are read-optimized views over that graph

## Update Rules

### Import Behavior

Each user import creates a new `import_batches` row before parsing starts.

For each raw mail record:

1. Insert a `message_sources` row tied to the import batch.
2. Normalize sender, recipients, and thread data.
3. Upsert `participants` by `normalized_email`.
4. Upsert or create `threads` by `thread_key`.
5. Upsert `messages` by `canonical_message_key`.
6. Link the `message_sources.message_id` to the canonical message.
7. Insert `message_participants` rows if the canonical message is new, or reconcile missing recipient-role rows if the existing canonical record was incomplete.

### Re-Import Idempotency

Re-importing the same mailbox should:

- create a new `import_batches` row for auditability
- create fresh `message_sources` rows for that batch
- reuse existing canonical `messages`, `participants`, and `threads` where keys match
- avoid duplicating `messages` rows when the same canonical key already exists

This gives repeatable import history without inflating the canonical mail graph.

### Metadata Refresh Rules

Canonical rows may update a small set of non-identity fields on re-import:

- `participants.display_name`
- `participants.last_seen_at`
- `threads.latest_message_at`
- `threads.message_count`
- `threads.participant_count`
- `messages.body_preview` if a richer preview is available

Identity fields must not change silently:

- `participants.normalized_email`
- `messages.canonical_message_key`
- `threads.thread_key`

### Rescoring Behavior

Rescoring must not mutate source-of-truth mail tables.

A rescoring pass should:

1. Insert a new `scoring_runs` row.
2. Compute new `participant_relationship_scores`.
3. Compute new `signal_scores`.
4. Generate new `briefing_entries`.
5. Mark the previous run's derived rows as `is_current = false`.
6. Mark the new run's rows as `is_current = true` in the same transaction if possible.

This keeps ranking history inspectable while ensuring the UI has a single current snapshot.

## Query Shapes

### Dashboard Read

The dashboard needs:

- current `signal_scores` ordered by `priority_score desc`
- thread metadata from `threads`
- most recent message preview from `messages`
- human-readable participants from sender and recipient links
- explanation details from `signal_scores.explanation_json`

Recommended read path:

1. fetch current `signal_scores`
2. join `threads`
3. join latest `messages` per thread using `thread_id, sent_at desc`
4. join participant identities as needed

### Morning Briefing Read

The morning briefing should read:

- current `briefing_entries` ordered by `rank_position`
- supporting thread metadata from `threads`
- optional latest message preview for drill-in

Because briefing entries persist per scoring run, the read path stays simple and does not require regeneration during view rendering.

## Decisions on Issue Open Questions

### Should participant-level `relationship_score` be stored directly, derived on demand, or both?

Store it as a derived snapshot in `participant_relationship_scores`, not on `participants`.

Why:

- avoids mixing stable identity with recomputable scoring state
- preserves scoring history across rubric changes
- supports rescoring without mutating canonical participant records

### Do briefing entries persist as snapshots or regenerate per read in Phase 1?

Persist snapshots in `briefing_entries`.

Why:

- makes the demo deterministic
- keeps the UI fast
- aligns with the broader rule that derived data is generated per scoring run

### What import-batch metadata is required for repeatability and debugging?

Minimum required metadata:

- source path and filename
- source file fingerprint
- file size
- parser version
- start and completion timestamps
- import status
- raw message counts
- linked canonical message counts
- parse error counts and summary notes

## Out of Scope

This document does not define:

- exact SQLite DDL syntax
- FTS strategy for search
- binary attachment storage
- AI enrichment tables beyond the current Phase 1 boundary
- migration procedures for future threading algorithm rewrites

Those can follow once the repo moves from architecture-definition mode into scaffolding and implementation.
