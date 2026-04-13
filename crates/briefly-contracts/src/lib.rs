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
}
