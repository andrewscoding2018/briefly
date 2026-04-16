"use client";

import { useState, useTransition } from "react";

import type { ImportBatchOutput } from "@/lib/contracts";
import { importMailbox, isDesktopRuntime, pickMailboxPath } from "@/lib/desktop";
import {
  getImportHeadline,
  initialImportFlowState,
  startMailboxImport,
  type ImportFlowState,
} from "@/lib/import-flow";

function formatTimestamp(value: string | null) {
  if (!value) {
    return "Unknown";
  }

  return new Intl.DateTimeFormat("en-US", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(value));
}

function StatusPanel({
  pending,
  state,
}: {
  pending: boolean;
  state: ImportFlowState;
}) {
  const batch = state.result?.batch ?? null;

  return (
    <section className="status-panel" aria-live="polite">
      <div className="status-header">
        <p className="eyebrow">Phase 1 Import</p>
        <h2>{getImportHeadline(pending ? "running" : state.lifecycle)}</h2>
      </div>
      <p className="status-copy">
        {pending
          ? "Rust is parsing the selected mailbox and normalizing message, participant, and thread records."
          : state.lifecycle === "idle"
            ? "Select a local .mbox archive to run the first desktop ingestion flow entirely on-device."
            : state.errorMessage ??
              "The mailbox finished importing and the desktop app has a normalized batch ready for the next scoring step."}
      </p>
      <dl className="status-metadata">
        <div>
          <dt>Selected file</dt>
          <dd>{state.selectedPath ?? "Nothing selected yet"}</dd>
        </div>
        <div>
          <dt>Lifecycle</dt>
          <dd>{pending ? "running" : state.lifecycle}</dd>
        </div>
        <div>
          <dt>Imported at</dt>
          <dd>{formatTimestamp(batch?.imported_at ?? null)}</dd>
        </div>
      </dl>
      {batch ? <ImportMetrics batch={batch} /> : null}
    </section>
  );
}

function ImportMetrics({ batch }: { batch: ImportBatchOutput }) {
  const topThreads = batch.threads.slice(0, 3);

  return (
    <div className="metrics">
      <article className="metric-card">
        <span>Messages seen</span>
        <strong>{batch.message_count_seen}</strong>
      </article>
      <article className="metric-card">
        <span>Accepted</span>
        <strong>{batch.accepted_messages.length}</strong>
      </article>
      <article className="metric-card">
        <span>Rejected</span>
        <strong>{batch.rejected_messages.length}</strong>
      </article>
      <article className="metric-card">
        <span>Threads</span>
        <strong>{batch.threads.length}</strong>
      </article>
      <section className="thread-preview">
        <h3>First normalized threads</h3>
        <ul>
          {topThreads.length > 0 ? (
            topThreads.map((thread) => (
              <li key={thread.thread_id}>
                <span>{thread.canonical_subject ?? "Untitled thread"}</span>
                <small>{thread.message_count} messages</small>
              </li>
            ))
          ) : (
            <li>
              <span>No thread previews available yet.</span>
            </li>
          )}
        </ul>
      </section>
    </div>
  );
}

export function ImportPage() {
  const [state, setState] = useState(initialImportFlowState);
  const [isPending, startTransition] = useTransition();
  const [desktopReady, setDesktopReady] = useState<boolean | null>(null);

  async function handleImport() {
    const desktop = await isDesktopRuntime();
    setDesktopReady(desktop);

    if (!desktop) {
      setState({
        lifecycle: "failed",
        selectedPath: null,
        errorMessage:
          "Desktop APIs are unavailable in the browser preview. Launch the Tauri shell to pick and import a mailbox.",
        result: null,
      });
      return;
    }

    startTransition(() => {
      void startMailboxImport(
        {
          pickMailboxPath,
          importMailbox,
        },
        setState,
      );
    });
  }

  return (
    <main className="page import-shell">
      <section className="hero import-hero">
        <div className="hero-copy">
          <p className="eyebrow">Briefly Desktop</p>
          <h1>Bring a local mailbox into Briefly and let Rust do the heavy lifting.</h1>
          <p className="lede">
            The first product surface is import-first on purpose: the app begins by
            selecting a local <code>.mbox</code> archive, handing the path to Tauri
            IPC, and letting the Rust ingest crate normalize the mailbox offline.
          </p>
        </div>
        <div className="hero-actions">
          <button className="import-button" onClick={handleImport} disabled={isPending}>
            {isPending ? "Import running..." : "Choose mailbox"}
          </button>
          <p className="helper-copy">
            {desktopReady === false
              ? "The browser preview can render the UI, but only the desktop shell can access native file dialogs."
              : "macOS file dialog selection happens through Tauri; the actual parsing stays in Rust."}
          </p>
        </div>
      </section>

      <section className="learning-grid" aria-label="Tauri and Rust learning notes">
        <article className="card">
          <h2>Tauri boundary</h2>
          <p>
            The frontend does not parse files directly. It asks Tauri to invoke a Rust
            command, which keeps native access and mailbox parsing on the system side.
          </p>
        </article>
        <article className="card">
          <h2>Rust ownership</h2>
          <p>
            The ingest crate returns a structured value instead of mutating UI state.
            That separation makes the parser reusable for tests, commands, and future
            persistence steps.
          </p>
        </article>
        <article className="card">
          <h2>Next step</h2>
          <p>
            Once import is stable, scoring and persistence can consume the same batch
            payload without redesigning the desktop boundary.
          </p>
        </article>
      </section>

      <StatusPanel pending={isPending} state={state} />
    </main>
  );
}
