export const bootstrapCards = [
  {
    title: "Desktop shell",
    description:
      "Next.js hosts the initial product surface while src-tauri stays as a stub IPC boundary."
  },
  {
    title: "Rust services",
    description:
      "Separate crates keep ingestion, storage, scoring, briefing, and AI adapters from collapsing into one binary."
  },
  {
    title: "Contracts",
    description:
      "Example payloads and shared types give the frontend and Rust layers one place to anchor interface changes."
  },
  {
    title: "Fixtures",
    description:
      "Mailbox, scoring, and UI fixtures are reserved early so tests can grow from seeded artifacts instead of one-off samples."
  }
] as const;
