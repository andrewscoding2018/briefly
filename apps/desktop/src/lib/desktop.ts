import type {
  DesktopImportResponse,
  FocusDashboardResponse,
} from "@/lib/contracts";

export async function isDesktopRuntime() {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export async function pickMailboxPath() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selection = await open({
    directory: false,
    multiple: false,
    filters: [{ name: "Mailbox", extensions: ["mbox"] }],
  });

  return typeof selection === "string" ? selection : null;
}

export async function importMailbox(path: string) {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<DesktopImportResponse>("import_mailbox", { path });
}

export async function loadFocusDashboard() {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<FocusDashboardResponse>("load_focus_dashboard");
}
