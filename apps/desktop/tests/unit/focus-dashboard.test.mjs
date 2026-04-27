import test from "node:test";
import assert from "node:assert/strict";

import {
  formatPriorityScore,
  getDashboardViewState,
  summarizeParticipants,
} from "../../src/lib/focus-dashboard.ts";

const readySnapshot = {
  generated_at: "2026-04-16T00:00:00Z",
  has_imported_mailbox: true,
  last_import_status: "completed",
  last_scoring_status: "completed",
  threads: [
    {
      thread_id: "thr_focus",
      canonical_subject: "Can you review the investor update?",
      latest_message_at: "2026-04-16T00:00:00Z",
      latest_message_preview: "Can you review the attached update by tomorrow?",
      message_count: 2,
      participants: [
        {
          participant_id: "par_founder",
          email: "founder@example.com",
          display_name: "Founder",
        },
        {
          participant_id: "par_operator",
          email: "operator@example.com",
          display_name: "Operator",
        },
      ],
      scores: {
        relationship_score: 0.82,
        actionability_score: 0.65,
        urgency_score: 0.4,
        recency_score: 0.8,
        priority_score: 0.69,
      },
      explanation: {
        version: "phase1_v1",
        top_reasons: ["Strong relationship with sender", "Direct ask detected"],
        component_scores: {
          relationship_score: 0.82,
          actionability_score: 0.65,
          urgency_score: 0.4,
          recency_score: 0.8,
          priority_score: 0.69,
        },
        matched_signals: ["participant_familiarity_high", "ask_language_present"],
        applied_penalties: [],
      },
    },
  ],
};

test("getDashboardViewState reports loading while snapshot is pending", () => {
  assert.equal(
    getDashboardViewState({
      isLoading: true,
      errorMessage: null,
      snapshot: null,
    }),
    "loading",
  );
});

test("getDashboardViewState distinguishes import-empty and scoring-empty states", () => {
  assert.equal(
    getDashboardViewState({
      isLoading: false,
      errorMessage: null,
      snapshot: {
        ...readySnapshot,
        has_imported_mailbox: false,
        threads: [],
      },
    }),
    "import-empty",
  );

  assert.equal(
    getDashboardViewState({
      isLoading: false,
      errorMessage: null,
      snapshot: {
        ...readySnapshot,
        has_imported_mailbox: true,
        threads: [],
      },
    }),
    "scoring-empty",
  );
});

test("getDashboardViewState reports error and ready states", () => {
  assert.equal(
    getDashboardViewState({
      isLoading: false,
      errorMessage: "dashboard failed",
      snapshot: readySnapshot,
    }),
    "error",
  );

  assert.equal(
    getDashboardViewState({
      isLoading: false,
      errorMessage: null,
      snapshot: readySnapshot,
    }),
    "ready",
  );
});

test("summarizeParticipants truncates long participant lists and priority scores round cleanly", () => {
  assert.equal(summarizeParticipants(readySnapshot.threads[0]), "Founder, Operator");
  assert.equal(formatPriorityScore(0.69), 69);
});
