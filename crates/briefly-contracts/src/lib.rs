use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapBoundary {
    pub title: &'static str,
    pub description: &'static str,
}

pub const BOOTSTRAP_BOUNDARIES: [BootstrapBoundary; 4] = [
    BootstrapBoundary {
        title: "Desktop shell",
        description: "Owns the product surface and future IPC command registration.",
    },
    BootstrapBoundary {
        title: "Ingestion",
        description: "Parses .mbox files and normalizes canonical mailbox entities.",
    },
    BootstrapBoundary {
        title: "Scoring",
        description: "Computes deterministic priority and explanation outputs.",
    },
    BootstrapBoundary {
        title: "Fixtures",
        description: "Seeds mailbox, scoring, and UI tests from shared artifacts.",
    },
];

pub fn bootstrap_banner() -> &'static str {
    "Briefly desktop shell bootstrap"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DesktopImportLifecycle {
    Running,
    Completed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DesktopImportResponse {
    pub lifecycle: DesktopImportLifecycle,
    pub selected_path: Option<String>,
    pub batch: Option<ImportBatchOutput>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportBatchOutput {
    pub import_batch_id: String,
    pub source_path: String,
    pub source_fingerprint: String,
    pub imported_at: String,
    pub parser_version: String,
    pub status: ImportBatchStatus,
    pub message_count_seen: usize,
    pub accepted_messages: Vec<NormalizedMessage>,
    pub rejected_messages: Vec<RejectedMessage>,
    pub participants: Vec<Participant>,
    pub threads: Vec<Thread>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportBatchStatus {
    Completed,
    Partial,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NormalizedMessage {
    pub message_key: String,
    pub raw_message_id: Option<String>,
    pub thread_id: String,
    pub subject: Option<String>,
    pub canonical_subject: Option<String>,
    pub sender_participant_id: String,
    pub sender: Participant,
    pub to: Vec<Participant>,
    pub cc: Vec<Participant>,
    pub bcc: Vec<Participant>,
    pub reply_to: Vec<Participant>,
    pub sent_at: Option<String>,
    pub body_text: Option<String>,
    pub body_preview: Option<String>,
    pub body_text_digest: Option<String>,
    pub has_html_body: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Participant {
    pub participant_id: String,
    pub email: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Thread {
    pub thread_id: String,
    pub canonical_subject: Option<String>,
    pub root_message_key: String,
    pub latest_message_at: Option<String>,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RejectedMessage {
    pub source_index: usize,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bootstrap_boundaries_are_defined() {
        assert_eq!(BOOTSTRAP_BOUNDARIES.len(), 4);
        assert!(BOOTSTRAP_BOUNDARIES
            .iter()
            .any(|boundary| boundary.title == "Ingestion"));
    }

    #[test]
    fn import_batch_status_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&ImportBatchStatus::Partial).unwrap(),
            "\"partial\""
        );
    }

    #[test]
    fn desktop_import_lifecycle_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&DesktopImportLifecycle::Running).unwrap(),
            "\"running\""
        );
    }
}
