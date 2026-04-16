import type { FocusDashboardResponse } from "@/lib/contracts";

export type DashboardViewState =
  | "loading"
  | "ready"
  | "import-empty"
  | "scoring-empty"
  | "error";

export function getDashboardViewState(input: {
  isLoading: boolean;
  errorMessage: string | null;
  snapshot: FocusDashboardResponse | null;
}): DashboardViewState {
  if (input.isLoading) {
    return "loading";
  }

  if (input.errorMessage) {
    return "error";
  }

  if (!input.snapshot || input.snapshot.threads.length === 0) {
    return input.snapshot?.has_imported_mailbox ? "scoring-empty" : "import-empty";
  }

  return "ready";
}

export function summarizeParticipants(snapshot: FocusDashboardResponse["threads"][number]) {
  const labels = snapshot.participants
    .slice(0, 3)
    .map((participant) => participant.display_name ?? participant.email);

  if (snapshot.participants.length <= 3) {
    return labels.join(", ");
  }

  return `${labels.join(", ")} +${snapshot.participants.length - 3} more`;
}

export function formatPriorityScore(score: number) {
  return Math.round(score * 100);
}
