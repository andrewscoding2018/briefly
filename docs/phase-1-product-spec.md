# Phase 1 Product Specification

## Purpose

This document defines the product target for Briefly's Phase 1 MVP so architecture and implementation work optimize for a single credible demo outcome instead of a generic inbox client.

## Primary MVP Persona

The Phase 1 user is an operator-founder or small-team executive who receives a high volume of mixed email but still personally handles critical conversations.

This persona is the best Phase 1 fit because:

- they have enough email volume for inbox overload to be obvious in a demo
- they still care about relationship context, pending asks, and follow-up risk
- they can export an `.mbox` archive without requiring enterprise IT integration
- they tolerate a read-only triage tool if it saves attention

Representative characteristics:

- mailbox includes founder, investor, customer, hiring, vendor, and system-generated traffic
- many threads are important because of who sent them, not because they contain explicit tasks
- the user wants to know what deserves attention first, not process every unread message

## Demo Dataset Shape

Phase 1 demos should use a realistic but manageable local mailbox archive:

- target dataset: 10,000 to 50,000 messages
- minimum credible demo dataset: 5,000 messages
- preferred thread count: 1,500 to 8,000 threads
- time horizon: 6 to 24 months of historical mail

Expected mailbox composition:

- 10% to 20% high-value human conversation
- 20% to 30% routine but still human-generated operational email
- 50% to 70% low-value bulk, marketing, receipts, alerts, and automated notifications

The demo dataset should include:

- active one-to-one conversations
- multi-party decision threads
- stale threads that still contain unresolved asks
- noisy automated mail that should rank low

Phase 1 does not need to prove correctness on massive enterprise mailboxes or highly regulated datasets.

## Signals-First Minimum Behavior

For v1, "signals-first" means the dashboard consistently elevates threads that matter because of relationship strength, recency, and likely actionability, while suppressing obvious background noise.

The minimum acceptable dashboard behavior is:

- show a ranked focus list of threads rather than a raw chronological inbox
- place clearly important human threads near the top even when newer low-value mail exists
- attach simple visible reasons for ranking, such as strong relationship, direct ask, recent reply, or active back-and-forth
- keep obvious bulk notifications, receipts, and machine-generated alerts out of the top results in normal demo datasets
- support a morning briefing that summarizes the top priorities and suggests what deserves review next

The MVP does not need perfect ranking. It needs to make the product thesis legible within a short demo:

- a user can open Briefly and quickly understand why the top threads are there
- the output feels directionally trustworthy without manual score explanation from the presenter
- the morning briefing reads like a triage aid, not a generic summary dump

## Phase 1 Non-Goals

Phase 1 explicitly does not aim to deliver:

- a full replacement for the user's primary email client
- live Gmail or IMAP sync
- outbound email composition, reply, send, or workflow automation
- team collaboration features, shared views, or assignment
- perfect thread reconstruction across every malformed mailbox edge case
- AI-dependent ranking, summarization, or extraction
- cross-platform polish beyond a macOS-first demo
- admin controls, enterprise security review, or compliance positioning

## MVP Success Metrics

Success metrics should be concrete enough to judge the MVP after implementation and during demos.

### Product Metrics

- In a seeded demo dataset, at least 7 of the top 10 threads should be judged high-value by a human reviewer familiar with the mailbox.
- In a seeded demo dataset, obviously low-value automated mail should make up no more than 2 of the top 10 ranked threads.
- A first-time viewer should be able to describe Briefly's value proposition from the dashboard and briefing within 2 minutes of use.

### Experience Metrics

- Import of a 10,000-message mailbox should complete on a developer laptop in under 2 minutes.
- The ranked dashboard should become interactively usable after import without requiring network access.
- Each top-ranked thread should expose enough explanation that a presenter can justify its placement in one sentence.

### Demo Metrics

- The morning briefing should produce at least 5 useful briefing entries from a realistic demo mailbox.
- In a live demo, the presenter should not need to manually rescue the product thesis by skipping over a top-heavy block of noise.
- The MVP should support a 5-minute end-to-end demo flow: import, score, review focus dashboard, open briefing.

## Scope Boundaries for Follow-On Work

This specification intentionally defines product target and demo expectations, but leaves lower-level ingestion, normalization, and scoring contract details to follow-up documentation and issues.
