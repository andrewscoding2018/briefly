"use client";

import { useEffect, useState, useTransition } from "react";

import type { FocusDashboardResponse } from "@/lib/contracts";
import {
  formatPriorityScore,
  getDashboardViewState,
  summarizeParticipants,
} from "@/lib/focus-dashboard";
import { importMailbox, isDesktopRuntime, loadFocusDashboard, pickMailboxPath } from "@/lib/desktop";
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

function ImportStatus({ pending, state }: { pending: boolean; state: ImportFlowState }) {
  return (
    <section className="dashboard-banner" aria-live="polite">
      <div>
        <p className="eyebrow">Import Status</p>
        <h2>{getImportHeadline(pending ? "running" : state.lifecycle)}</h2>
      </div>
      <dl className="status-metadata compact">
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
          <dd>{formatTimestamp(state.result?.batch?.imported_at ?? null)}</dd>
        </div>
      </dl>
      {state.errorMessage ? <p className="banner-copy">{state.errorMessage}</p> : null}
    </section>
  );
}

function EmptyState({
  title,
  copy,
}: {
  title: string;
  copy: string;
}) {
  return (
    <section className="empty-state">
      <h3>{title}</h3>
      <p>{copy}</p>
    </section>
  );
}

function DashboardGrid({ snapshot }: { snapshot: FocusDashboardResponse }) {
  return (
    <section className="dashboard-grid" aria-label="Ranked focus threads">
      {snapshot.threads.map((thread, index) => (
        <article className="thread-card" key={thread.thread_id}>
          <div className="thread-card-header">
            <div>
              <span className="thread-rank">#{index + 1}</span>
              <h3>{thread.canonical_subject ?? "Untitled thread"}</h3>
            </div>
            <div className="score-pill">{formatPriorityScore(thread.scores.priority_score)}</div>
          </div>
          <p className="thread-meta">
            {summarizeParticipants(thread)} · {thread.message_count} messages · updated{" "}
            {formatTimestamp(thread.latest_message_at)}
          </p>
          <p className="thread-preview-copy">
            {thread.latest_message_preview ?? "No message preview available yet."}
          </p>
          <ul className="reason-list">
            {thread.explanation.top_reasons.map((reason) => (
              <li key={reason}>{reason}</li>
            ))}
          </ul>
          <dl className="score-grid">
            <div>
              <dt>Relationship</dt>
              <dd>{formatPriorityScore(thread.scores.relationship_score)}</dd>
            </div>
            <div>
              <dt>Actionability</dt>
              <dd>{formatPriorityScore(thread.scores.actionability_score)}</dd>
            </div>
            <div>
              <dt>Urgency</dt>
              <dd>{formatPriorityScore(thread.scores.urgency_score)}</dd>
            </div>
            <div>
              <dt>Recency</dt>
              <dd>{formatPriorityScore(thread.scores.recency_score)}</dd>
            </div>
          </dl>
        </article>
      ))}
    </section>
  );
}

export function ImportPage() {
  const [state, setState] = useState(initialImportFlowState);
  const [isPending, startTransition] = useTransition();
  const [desktopReady, setDesktopReady] = useState<boolean | null>(null);
  const [dashboard, setDashboard] = useState<FocusDashboardResponse | null>(null);
  const [dashboardError, setDashboardError] = useState<string | null>(null);
  const [isDashboardLoading, setIsDashboardLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;

    async function bootDashboard() {
      const desktop = await isDesktopRuntime();
      if (cancelled) {
        return;
      }

      setDesktopReady(desktop);
      if (!desktop) {
        setIsDashboardLoading(false);
        return;
      }

      try {
        const snapshot = await loadFocusDashboard();
        if (!cancelled) {
          setDashboard(snapshot);
          setDashboardError(null);
        }
      } catch (error) {
        if (!cancelled) {
          setDashboardError(
            error instanceof Error ? error.message : "Focus dashboard failed to load.",
          );
        }
      } finally {
        if (!cancelled) {
          setIsDashboardLoading(false);
        }
      }
    }

    void bootDashboard();

    return () => {
      cancelled = true;
    };
  }, []);

  async function handleImport() {
    const desktop = await isDesktopRuntime();
    setDesktopReady(desktop);

    if (!desktop) {
      setState({
        lifecycle: "failed",
        selectedPath: null,
        errorMessage:
          "Desktop APIs are unavailable in the browser preview. Launch the Tauri shell to import a mailbox and score ranked threads.",
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
        (nextState) => {
          setState(nextState);

          if (nextState.lifecycle === "completed" || nextState.lifecycle === "partial") {
            setIsDashboardLoading(true);
            void loadFocusDashboard()
              .then((snapshot) => {
                setDashboard(snapshot);
                setDashboardError(null);
              })
              .catch((error) => {
                setDashboardError(
                  error instanceof Error ? error.message : "Focus dashboard failed to refresh.",
                );
              })
              .finally(() => {
                setIsDashboardLoading(false);
              });
          }
        },
      );
    });
  }

  const dashboardState = getDashboardViewState({
    isLoading: isDashboardLoading,
    errorMessage: dashboardError,
    snapshot: dashboard,
  });

  return (
    <main className="page dashboard-shell">
      <section className="hero dashboard-hero">
        <div className="hero-copy">
          <p className="eyebrow">Briefly Focus Dashboard</p>
          <h1>Rank threads by attention, not chronology.</h1>
          <p className="lede">
            The desktop shell now reads current ranked threads from the local store,
            ordered by <code>priority_score</code> with visible explanation payloads so
            users can see why a conversation surfaced.
          </p>
        </div>
        <div className="hero-actions">
          <button className="import-button" onClick={handleImport} disabled={isPending}>
            {isPending ? "Import running..." : "Import mailbox"}
          </button>
          <p className="helper-copy">
            {desktopReady === false
              ? "Browser preview can show the shell, but only the macOS Tauri runtime can persist imports and score the dashboard."
              : "Import persists canonical messages, runs deterministic scoring, and refreshes the focus list from SQLite."}
          </p>
          <p className="helper-copy subtle">
            Last refresh: {formatTimestamp(dashboard?.generated_at ?? null)}
          </p>
        </div>
      </section>

      <ImportStatus pending={isPending} state={state} />

      {dashboardState === "loading" ? (
        <EmptyState
          title="Loading ranked threads"
          copy="Briefly is reading the current scoring snapshot from the local store."
        />
      ) : null}

      {dashboardState === "error" ? (
        <EmptyState
          title="Dashboard unavailable"
          copy={dashboardError ?? "Focus dashboard failed to load from the local store."}
        />
      ) : null}

      {dashboardState === "import-empty" ? (
        <EmptyState
          title="Import a mailbox to start ranking"
          copy="No persisted mailbox data exists yet, so there are no focus threads to rank."
        />
      ) : null}

      {dashboardState === "scoring-empty" ? (
        <EmptyState
          title="Mailbox imported, but no ranked threads are available"
          copy="Import succeeded, but the current scoring snapshot is empty. Re-run import or inspect the scoring pipeline."
        />
      ) : null}

      {dashboardState === "ready" && dashboard ? <DashboardGrid snapshot={dashboard} /> : null}
    </main>
  );
}
