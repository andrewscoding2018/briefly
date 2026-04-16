import type {
  DesktopImportLifecycle,
  DesktopImportResponse,
} from "@/lib/contracts";

export type ImportClient = {
  pickMailboxPath: () => Promise<string | null>;
  importMailbox: (path: string) => Promise<DesktopImportResponse>;
};

export type ImportFlowState = {
  lifecycle: DesktopImportLifecycle;
  selectedPath: string | null;
  errorMessage: string | null;
  result: DesktopImportResponse | null;
};

export const initialImportFlowState: ImportFlowState = {
  lifecycle: "idle",
  selectedPath: null,
  errorMessage: null,
  result: null,
};

export async function startMailboxImport(
  client: ImportClient,
  onStateChange: (state: ImportFlowState) => void,
) {
  const selectedPath = await client.pickMailboxPath();

  if (!selectedPath) {
    return initialImportFlowState;
  }

  onStateChange({
    lifecycle: "running",
    selectedPath,
    errorMessage: null,
    result: null,
  });

  try {
    const result = await client.importMailbox(selectedPath);
    const nextState: ImportFlowState = {
      lifecycle: result.lifecycle,
      selectedPath: result.selected_path,
      errorMessage: result.error_message,
      result,
    };
    onStateChange(nextState);
    return nextState;
  } catch (error) {
    const message =
      error instanceof Error
        ? error.message
        : "Mailbox import failed for an unknown reason.";
    const nextState: ImportFlowState = {
      lifecycle: "failed",
      selectedPath,
      errorMessage: message,
      result: null,
    };
    onStateChange(nextState);
    return nextState;
  }
}

export function getImportHeadline(lifecycle: DesktopImportLifecycle) {
  switch (lifecycle) {
    case "running":
      return "Importing mailbox";
    case "completed":
      return "Import completed";
    case "partial":
      return "Import completed with warnings";
    case "failed":
      return "Import failed";
    case "idle":
    default:
      return "Import your mailbox";
  }
}
