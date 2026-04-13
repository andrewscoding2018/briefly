# Phase 1 Mailbox Ingestion and Normalization Specification

## Status

Proposed baseline for issue #2.

## Purpose

This document defines the Phase 1 contract for turning a user-selected `.mbox`
file into normalized mailbox entities that Briefly can score and render. It
resolves the ingestion-specific gaps left intentionally open in the broader
implementation plan.

## Scope

Phase 1 ingestion is limited to:

- a single local `.mbox` file selected by the user
- deterministic local parsing and normalization
- stable identities for imported messages, participants, and threads
- best-effort recovery from malformed content

Phase 1 ingestion explicitly excludes:

- live IMAP, Gmail API, or Maildir ingestion
- attachment extraction beyond metadata needed for parsing safety
- remote enrichment or model-based cleanup during import
- lossless preservation of every raw MIME detail in the normalized store

## Input Assumptions

The importer assumes the selected source is a mailbox file that can be read as
an RFC 4155-style `.mbox` stream with messages separated by `From ` envelope
lines. Phase 1 should accept common `mboxo` and `mboxrd` variants when they can
be segmented with the standard delimiter rules.

The importer must treat each file selection as one `ImportBatch`. A batch may
contain zero or more valid messages and may complete with recoverable per-message
errors.

Phase 1 support assumptions:

- the input is one mailbox file, not a directory tree
- the file is readable from the local filesystem at import time
- messages may contain MIME multipart bodies
- text content may be UTF-8 or another declared charset
- the mailbox may contain malformed headers, missing IDs, or duplicate messages

Unsupported inputs should fail early at the batch level with a user-visible
error:

- directories or Maildir-style folder layouts
- archives that require decompression before parsing
- mailbox files whose envelope boundaries cannot be segmented at all

## Import Batch Semantics

Repeated imports create new `ImportBatch` records instead of overwriting past
batch metadata. The normalized mailbox entities remain stable across imports.

Phase 1 import behavior is:

- create a new `ImportBatch` for every user-triggered import
- compute a source fingerprint for the selected file using file hash and path
- upsert normalized `Message`, `Participant`, and `Thread` entities
- record batch-specific provenance separately from canonical entities
- mark duplicate observations instead of creating duplicate canonical messages

This gives Briefly stable identities for scoring while preserving the audit trail
of when a message was seen again in a later import.

## Parsing Expectations

For each segmented raw message, the importer must:

1. parse RFC 5322-style headers with line folding support
2. decode encoded-word headers when possible
3. parse MIME structure well enough to extract a normalized text body
4. normalize mailbox addresses from `From`, `Sender`, `Reply-To`, `To`, `Cc`,
   and `Bcc` when present
5. preserve raw values needed for traceability when normalization fails

Body extraction rules:

- prefer `text/plain` when available
- if only `text/html` is available, convert it to plain text
- ignore binary attachment contents for Phase 1 scoring
- decode text using the declared charset when valid
- fall back to UTF-8 with replacement characters on undecodable content

## Minimum Acceptance Rules

A segmented message is accepted into the normalized store only when the importer
can derive all of the following:

- at least one sender identity from `From` or `Sender`
- at least one stable message key, using `Message-ID` or the fallback fingerprint
- at least one content signal, meaning a subject, a normalized body, or both

`Date` is not required for acceptance. When `Date` is missing or invalid, the
message remains ingestible with a null timestamp and degraded ordering quality.

Messages that fail the minimum acceptance rules do not abort the batch. They are
recorded as rejected items in batch diagnostics with the reason for rejection.

## Message Identity and Deduplication

Canonical message identity follows this order:

1. use normalized `Message-ID` when present
2. otherwise generate a deterministic fallback fingerprint

`Message-ID` normalization for Phase 1:

- trim surrounding whitespace
- strip a single surrounding pair of angle brackets
- preserve the remaining value exactly

Fallback fingerprint inputs:

- normalized sender email
- normalized subject
- normalized sent timestamp when available
- normalized body text digest

The fallback fingerprint exists only to support malformed mailboxes. A valid
`Message-ID` always wins over a fallback key.

Deduplication policy:

- if the canonical message key already exists, upsert the existing `Message`
- if the same message appears in a later batch, create a new provenance record
- if duplicate messages with the same key appear in one batch, keep one canonical
  `Message` and mark the others as duplicate observations
- if duplicates conflict, prefer the version with the most complete parseable
  metadata and non-empty normalized body

Phase 1 must not create separate canonical messages just because the same email
was imported twice.

## Malformed and Partial Message Handling

Malformed content should degrade message quality, not batch success.

Required behavior:

- invalid header lines are skipped and recorded in diagnostics
- missing `Subject` becomes `null`
- missing `Date` becomes `null`
- missing `Message-ID` triggers fallback fingerprint generation
- undecodable text is stored with replacement characters
- unknown charsets fall back to UTF-8 replacement decoding
- malformed MIME parts are skipped when sibling parts still yield usable text

If no usable text body can be extracted, the message may still be accepted when a
sender, stable key, and subject are present.

## Thread Reconstruction

Thread assignment uses a deterministic priority order.

Primary strategy:

1. if `In-Reply-To` references a known message, join that message's thread
2. otherwise, if the newest resolvable entry in `References` points to a known
   message, join that thread

Fallback strategy when reply headers are absent or unusable:

1. derive a canonical subject by removing localized reply/forward prefixes such
   as `Re:`, `Fwd:`, and repeated variants
2. search for an existing thread with the same canonical subject
3. require participant overlap between the candidate message and thread
4. require the candidate thread's latest known message to be within 14 days of
   the new message, or within 14 days of import time when the new message lacks
   a timestamp
5. otherwise create a new thread

This fallback intentionally prefers under-grouping over aggressive merges. Phase
1 should avoid joining unrelated bulk mail solely because subjects match.

## Normalized Output Entities

Phase 1 ingestion produces the following normalized entities and relationships.

### `ImportBatch`

One record per user-triggered import.

Required fields:

- `import_batch_id`
- `source_path`
- `source_fingerprint`
- `imported_at`
- `parser_version`
- `status`

### `Message`

One canonical record per logical email after deduplication.

Required fields:

- `message_key`
- `raw_message_id`
- `thread_id`
- `subject`
- `canonical_subject`
- `sender_participant_id`
- `sent_at`
- `body_text`
- `body_text_digest`
- `has_html_body`

### `Participant`

One canonical record per normalized email address.

Required fields:

- `participant_id`
- `email`
- `display_name`

Participant identity is keyed by normalized email address. Display names may be
updated when later imports provide a better value, but the email address remains
the stable identity anchor.

### `Thread`

One canonical conversation grouping.

Required fields:

- `thread_id`
- `canonical_subject`
- `root_message_key`
- `latest_message_at`
- `message_count`

### Relationship Records

Phase 1 should also persist relationship records needed for provenance and role
tracking:

- `ImportBatchMessageObservation` linking a canonical message to a batch
- `MessageParticipant` linking messages to participants with roles such as
  `from`, `to`, `cc`, `bcc`, and `reply_to`

These relationship records are part of the ingestion contract even if their
concrete table names change during implementation.

## Ingestion Output Guarantees

After a successful batch import:

- every accepted message has a stable canonical message key
- every accepted message belongs to exactly one thread
- every participant-linked address is normalized into a canonical participant
- repeated imports do not create duplicate canonical messages
- rejected messages are traceable through batch diagnostics

## Deferred Details

This specification leaves the following implementation details open:

- exact database schema and index definitions
- exact subject-prefix normalization rules beyond the required examples
- exact file-hash algorithm for `source_fingerprint`
- exact shape of user-visible import diagnostics

Those details may evolve during scaffolding as long as they preserve the
behavioral contract defined here.
