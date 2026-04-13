# Phase 1 Signal Scoring Rubric

## Status

Proposed baseline for issue #4.

## Purpose

This document defines the deterministic Phase 1 scoring rubric for ranking email
threads by relationship strength and actionable intent. It translates Briefly's
signals-first product direction into a concrete scoring contract that works
without AI and produces explanations suitable for the dashboard.

## Design Goals

The Phase 1 rubric must:

- rank threads using only normalized local mailbox data
- be deterministic and repeatable across runs on the same dataset
- elevate high-value human conversations over bulk or machine-generated mail
- expose compact, human-readable reasons for each score
- support rescoring when heuristics change without mutating canonical mail data

The rubric does not attempt perfect semantic understanding. It only needs to be
directionally trustworthy for a demo-quality triage experience.

## Scoring Model

Phase 1 computes four normalized component scores per thread on a `0.0` to
`1.0` scale:

- `relationship_score`
- `actionability_score`
- `urgency_score`
- `recency_score`

`priority_score` is then derived as a weighted sum of those components plus a
small thread-activity modifier and a bulk-noise penalty.

All score outputs should be rounded to 4 decimal places before persistence so
reruns remain stable across platforms.

## Component Definitions

### `relationship_score`

`relationship_score` estimates how strong the user's relationship is with the
participants in a thread based on mailbox history alone.

For each non-self participant in the thread, compute a participant familiarity
score and use the maximum value plus a small multi-party boost:

`relationship_score = clamp(max_participant_familiarity + multi_party_boost, 0.0, 1.0)`

Participant familiarity should be computed from these signals:

- `0.35` for total historical interaction volume with the participant
- `0.25` for reply reciprocity, meaning both sides have sent messages in the same thread history
- `0.20` for directness, meaning the participant frequently appears in `From` or direct `To` rather than only `Cc`
- `0.20` for historical recency of interaction with that participant

Recommended normalization rules:

- interaction volume saturates at 12 historical messages with the participant
- reciprocity is `1.0` after at least one reply from each side, otherwise `0.0`
- directness is `1.0` when the participant commonly appears in one-to-one or small-group mail, `0.5` for mixed direct and copied history, `0.0` when mostly copied
- historical recency is `1.0` within 7 days, `0.7` within 30 days, `0.4` within 90 days, else `0.1`

`multi_party_boost` should be:

- `0.05` when the thread has 3 to 5 distinct human participants
- `0.00` otherwise

The boost exists because small-group decision threads often matter, but it must
stay small so large recipient lists do not look automatically important.

### `actionability_score`

`actionability_score` estimates whether the thread likely contains a request,
decision, follow-up, or next step that deserves attention.

Compute it from message-text and metadata heuristics:

- `0.35` for direct ask language
- `0.20` for follow-up language
- `0.20` for explicit scheduling or deadline cues
- `0.15` for unanswered inbound state
- `0.10` for attachment or document-review cues

Recommended detection rules:

- direct ask language: phrases such as `can you`, `could you`, `please`, `need you to`, `let me know`, `what do you think`, `would you`, `able to`
- follow-up language: phrases such as `following up`, `checking in`, `bumping this`, `circling back`, `gentle reminder`
- scheduling or deadline cues: phrases such as `by friday`, `tomorrow`, `next week`, `before`, `deadline`, `ETA`, `schedule`, `meeting`, `calendar`
- unanswered inbound state: latest message is inbound, thread contains prior exchange history, and no later self-authored reply exists
- attachment or document-review cues: latest message has attachments or mentions `attached`, `deck`, `doc`, `proposal`, `contract`, `invoice`, `resume`

Text matching should be case-insensitive and based on normalized plaintext.
Multiple hits in the same category should saturate at that category's maximum
weight.

### `urgency_score`

`urgency_score` estimates whether the thread appears time-sensitive.

Compute it from:

- `0.40` for explicit time-bound language in the latest two messages
- `0.30` for rapid recent back-and-forth activity
- `0.20` for the latest message being an inbound follow-up after prior silence
- `0.10` for same-day freshness

Normalization guidance:

- explicit time-bound language is `1.0` when deadline phrases are found, else `0.0`
- rapid back-and-forth is `1.0` when there are at least 3 messages in the last 48 hours, `0.5` for 2 messages, else `0.0`
- inbound follow-up after silence is `1.0` when the latest inbound message includes follow-up language and the previous message was older than 3 days
- same-day freshness is `1.0` when the latest message is less than 24 hours old, else `0.0`

### `recency_score`

`recency_score` keeps active threads visible without letting raw chronology
dominate the ranking.

Use a step function based on `threads.latest_message_at`:

- `1.00` within 24 hours
- `0.80` within 3 days
- `0.60` within 7 days
- `0.35` within 14 days
- `0.15` within 30 days
- `0.05` older than 30 days or missing timestamp

## Priority Formula

Phase 1 should compute:

`priority_score = clamp(base_score + thread_activity_bonus - bulk_penalty, 0.0, 1.0)`

Where:

`base_score = relationship_score * 0.40 + actionability_score * 0.30 + urgency_score * 0.20 + recency_score * 0.10`

`thread_activity_bonus` should be:

- `0.05` when the thread has at least 4 messages and at least 2 distinct human participants
- `0.00` otherwise

`bulk_penalty` should be:

- `0.25` when any bulk-noise rule matches
- `0.10` when the thread looks partially noisy but still human-generated
- `0.00` otherwise

Bulk-noise rules should match obvious low-value threads, including:

- sender domain or headers suggesting automated mail such as `no-reply`, `noreply`, `notifications`, `mailer-daemon`
- subjects containing patterns such as `receipt`, `invoice paid`, `password reset`, `verify your email`, `newsletter`, `unsubscribe`
- recipient count greater than 10 with no reply chain evidence

The penalty should be applied after the positive score is computed so a noisy
thread is actively suppressed instead of merely failing to gain positive points.

## Explainability Contract

Every persisted `signal_scores` row must include an `explanation_json` payload
that supports UI rendering without recomputation.

Minimum shape:

```json
{
  "version": "phase1_v1",
  "top_reasons": [
    "Strong relationship with sender",
    "Direct ask detected",
    "Recent reply activity"
  ],
  "component_scores": {
    "relationship_score": 0.82,
    "actionability_score": 0.65,
    "urgency_score": 0.40,
    "recency_score": 0.80,
    "priority_score": 0.69
  },
  "matched_signals": [
    "participant_familiarity_high",
    "ask_language_present",
    "recent_back_and_forth"
  ],
  "applied_penalties": [
    "bulk_noise_partial"
  ]
}
```

UI requirements:

- show 2 to 3 `top_reasons` per ranked thread
- reasons must be plain-language and presenter-friendly
- reasons should reflect both positive signals and any meaningful penalty
- the UI should not expose raw keyword matches unless needed for debugging

Reason generation guidance:

- `Strong relationship with sender` when `relationship_score >= 0.70`
- `Direct ask detected` when the direct-ask category contributes at least `0.20`
- `Recent reply activity` when rapid back-and-forth contributes at least `0.15`
- `Awaiting your response` when unanswered inbound state contributes at least `0.15`
- `Time-sensitive language detected` when urgency includes a deadline cue
- `Likely bulk or automated mail` when any bulk penalty is applied

## Scoring Sequence

For each scoring run:

1. Compute participant-level familiarity snapshots from the full canonical message history.
2. Compute per-thread component scores using current normalized messages and threads.
3. Apply bonuses and penalties.
4. Persist `signal_scores` and `explanation_json`.
5. Mark the new run as current only after all thread scores are written successfully.

This ordering ensures thread scores can depend on stable participant-level
history without mutating the underlying canonical entities.

## Repeated Imports and Rescoring

Repeated imports and rubric changes must behave differently:

- repeated imports may add new canonical messages or new provenance records, then trigger a fresh scoring run
- rescoring with the same `scoring_version` on unchanged canonical data should produce identical scores
- rescoring with a new `scoring_version` may change only derived tables, never source-of-truth mail entities
- historical `signal_scores` rows should remain queryable by `scoring_run_id`

Phase 1 rule:

- a new import batch should trigger a new scoring run after normalization completes
- a rubric or weighting change must increment `scoring_runs.scoring_version`
- the UI should read only rows where `is_current = true`

## Edge Cases

When source data is incomplete, degrade gracefully:

- if `latest_message_at` is missing, use the minimum recency bucket
- if thread participants cannot be confidently resolved, compute relationship from any known sender only
- if body text is empty, actionability and urgency may still use subject text and attachment metadata
- if all positive signals are weak and bulk-noise rules match, the thread should remain near the bottom of the dashboard

## Initial Calibration Guidance

The rubric should be considered successful for Phase 1 when:

- clearly important human threads usually outrank bulk notifications in seeded demo data
- top-ranked threads can be explained in one sentence from `explanation_json`
- a human reviewer can inspect the top 10 results and judge at least 7 as worthwhile

If calibration fails, adjust weights and keyword lists first before adding more
complex heuristics. Phase 1 should stay legible and deterministic.
