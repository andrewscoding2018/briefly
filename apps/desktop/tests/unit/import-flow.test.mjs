import test from "node:test";
import assert from "node:assert/strict";

import {
  initialImportFlowState,
  startMailboxImport,
} from "../../src/lib/import-flow.ts";

test("startMailboxImport reports running then completed on success", async () => {
  const states = [];

  const result = await startMailboxImport(
    {
      pickMailboxPath: async () => "/tmp/mailbox.mbox",
      importMailbox: async (path) => ({
        lifecycle: "completed",
        selected_path: path,
        batch: {
          import_batch_id: "bat_123",
          source_path: path,
          source_fingerprint: "src_123",
          imported_at: "2026-04-16T00:00:00Z",
          parser_version: "briefly-ingest/0.1.0",
          status: "completed",
          message_count_seen: 2,
          accepted_messages: [],
          rejected_messages: [],
          participants: [],
          threads: [],
        },
        error_message: null,
      }),
    },
    (state) => states.push(state),
  );

  assert.equal(states[0].lifecycle, "running");
  assert.equal(states[0].selectedPath, "/tmp/mailbox.mbox");
  assert.equal(states[1].lifecycle, "completed");
  assert.equal(result.lifecycle, "completed");
});

test("startMailboxImport reports failed state when import throws", async () => {
  const states = [];

  const result = await startMailboxImport(
    {
      pickMailboxPath: async () => "/tmp/bad-input",
      importMailbox: async () => {
        throw new Error("directories and Maildir layouts are not supported");
      },
    },
    (state) => states.push(state),
  );

  assert.equal(states[0].lifecycle, "running");
  assert.equal(states[1].lifecycle, "failed");
  assert.match(states[1].errorMessage, /not supported/);
  assert.equal(result.lifecycle, "failed");
});

test("startMailboxImport leaves state unchanged when selection is cancelled", async () => {
  const states = [];

  const result = await startMailboxImport(
    {
      pickMailboxPath: async () => null,
      importMailbox: async () => {
        throw new Error("should not be called");
      },
    },
    (state) => states.push(state),
  );

  assert.deepEqual(states, []);
  assert.deepEqual(result, initialImportFlowState);
});
