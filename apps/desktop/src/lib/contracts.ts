export type ImportBatchStatus = "completed" | "partial" | "failed";

export type DesktopImportLifecycle =
  | "idle"
  | "running"
  | "completed"
  | "partial"
  | "failed";

export type Participant = {
  participant_id: string;
  email: string;
  display_name: string | null;
};

export type NormalizedMessage = {
  message_key: string;
  raw_message_id: string | null;
  thread_id: string;
  subject: string | null;
  canonical_subject: string | null;
  sender_participant_id: string;
  sender: Participant;
  to: Participant[];
  cc: Participant[];
  bcc: Participant[];
  reply_to: Participant[];
  sent_at: string | null;
  body_text: string | null;
  body_preview: string | null;
  body_text_digest: string | null;
  has_html_body: boolean;
};

export type Thread = {
  thread_id: string;
  canonical_subject: string | null;
  root_message_key: string;
  latest_message_at: string | null;
  message_count: number;
};

export type RejectedMessage = {
  source_index: number;
  reason: string;
};

export type ImportBatchOutput = {
  import_batch_id: string;
  source_path: string;
  source_fingerprint: string;
  imported_at: string;
  parser_version: string;
  status: ImportBatchStatus;
  message_count_seen: number;
  accepted_messages: NormalizedMessage[];
  rejected_messages: RejectedMessage[];
  participants: Participant[];
  threads: Thread[];
};

export type DesktopImportResponse = {
  lifecycle: Exclude<DesktopImportLifecycle, "idle">;
  selected_path: string | null;
  batch: ImportBatchOutput | null;
  error_message: string | null;
};

export type ScoringRunStatus = "running" | "completed" | "failed";

export type ThreadComponentScores = {
  relationship_score: number;
  actionability_score: number;
  urgency_score: number;
  recency_score: number;
  priority_score: number;
};

export type ScoreExplanationPayload = {
  version: string;
  top_reasons: string[];
  component_scores: ThreadComponentScores;
  matched_signals: string[];
  applied_penalties: string[];
};

export type RankedThreadCard = {
  thread_id: string;
  canonical_subject: string | null;
  latest_message_at: string | null;
  latest_message_preview: string | null;
  message_count: number;
  participants: Participant[];
  scores: ThreadComponentScores;
  explanation: ScoreExplanationPayload;
};

export type FocusDashboardResponse = {
  generated_at: string | null;
  has_imported_mailbox: boolean;
  last_import_status: ImportBatchStatus | null;
  last_scoring_status: ScoringRunStatus | null;
  threads: RankedThreadCard[];
};
