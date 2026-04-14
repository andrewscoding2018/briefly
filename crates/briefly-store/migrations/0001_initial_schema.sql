PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS import_batches (
    import_batch_id TEXT PRIMARY KEY,
    source_path TEXT NOT NULL,
    source_filename TEXT NOT NULL,
    source_sha256 TEXT NOT NULL,
    file_size_bytes INTEGER NOT NULL,
    parser_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'partial', 'failed')),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    message_count_seen INTEGER NOT NULL DEFAULT 0,
    message_count_linked INTEGER NOT NULL DEFAULT 0,
    parse_error_count INTEGER NOT NULL DEFAULT 0,
    notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_import_batches_source_sha256
    ON import_batches (source_sha256);
CREATE INDEX IF NOT EXISTS idx_import_batches_started_at_desc
    ON import_batches (started_at DESC);

CREATE TABLE IF NOT EXISTS threads (
    thread_id TEXT PRIMARY KEY,
    thread_key TEXT NOT NULL UNIQUE,
    canonical_subject TEXT,
    latest_message_at TEXT,
    message_count INTEGER NOT NULL DEFAULT 0,
    participant_count INTEGER NOT NULL DEFAULT 0,
    threading_version TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_threads_latest_message_at_desc
    ON threads (latest_message_at DESC);

CREATE TABLE IF NOT EXISTS participants (
    participant_id TEXT PRIMARY KEY,
    normalized_email TEXT NOT NULL UNIQUE,
    display_name TEXT,
    organization_hint TEXT,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_participants_last_seen_at_desc
    ON participants (last_seen_at DESC);

CREATE TABLE IF NOT EXISTS messages (
    message_id TEXT PRIMARY KEY,
    canonical_message_key TEXT NOT NULL UNIQUE,
    internet_message_id TEXT,
    thread_id TEXT NOT NULL,
    subject TEXT,
    normalized_subject TEXT,
    sent_at TEXT,
    sender_participant_id TEXT,
    body_text TEXT,
    body_preview TEXT,
    has_attachments INTEGER NOT NULL CHECK (has_attachments IN (0, 1)),
    import_first_seen_batch_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES threads (thread_id),
    FOREIGN KEY (sender_participant_id) REFERENCES participants (participant_id),
    FOREIGN KEY (import_first_seen_batch_id) REFERENCES import_batches (import_batch_id)
);

CREATE INDEX IF NOT EXISTS idx_messages_thread_sent_at_desc
    ON messages (thread_id, sent_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_sender_participant_id
    ON messages (sender_participant_id);
CREATE INDEX IF NOT EXISTS idx_messages_internet_message_id
    ON messages (internet_message_id);

CREATE TABLE IF NOT EXISTS message_sources (
    source_record_id TEXT PRIMARY KEY,
    import_batch_id TEXT NOT NULL,
    message_id TEXT,
    mailbox_path TEXT,
    source_position INTEGER,
    raw_message_sha256 TEXT NOT NULL,
    header_blob TEXT,
    parse_status TEXT NOT NULL CHECK (parse_status IN ('parsed', 'partial', 'failed')),
    parse_error TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (import_batch_id) REFERENCES import_batches (import_batch_id),
    FOREIGN KEY (message_id) REFERENCES messages (message_id)
);

CREATE INDEX IF NOT EXISTS idx_message_sources_import_batch_position
    ON message_sources (import_batch_id, source_position);
CREATE INDEX IF NOT EXISTS idx_message_sources_raw_message_sha256
    ON message_sources (raw_message_sha256);
CREATE INDEX IF NOT EXISTS idx_message_sources_message_id
    ON message_sources (message_id);

CREATE TABLE IF NOT EXISTS message_participants (
    message_id TEXT NOT NULL,
    participant_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('to', 'cc', 'bcc', 'reply_to')),
    position INTEGER NOT NULL,
    PRIMARY KEY (message_id, participant_id, role),
    FOREIGN KEY (message_id) REFERENCES messages (message_id),
    FOREIGN KEY (participant_id) REFERENCES participants (participant_id)
);

CREATE INDEX IF NOT EXISTS idx_message_participants_participant_role
    ON message_participants (participant_id, role);

CREATE TABLE IF NOT EXISTS scoring_runs (
    scoring_run_id TEXT PRIMARY KEY,
    trigger_import_batch_id TEXT,
    scoring_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('running', 'completed', 'failed')),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    notes TEXT,
    FOREIGN KEY (trigger_import_batch_id) REFERENCES import_batches (import_batch_id)
);

CREATE INDEX IF NOT EXISTS idx_scoring_runs_started_at_desc
    ON scoring_runs (started_at DESC);
CREATE INDEX IF NOT EXISTS idx_scoring_runs_scoring_version
    ON scoring_runs (scoring_version);

CREATE TABLE IF NOT EXISTS participant_relationship_scores (
    participant_id TEXT NOT NULL,
    scoring_run_id TEXT NOT NULL,
    relationship_score REAL NOT NULL,
    interaction_count INTEGER NOT NULL DEFAULT 0,
    last_interaction_at TEXT,
    is_current INTEGER NOT NULL CHECK (is_current IN (0, 1)),
    computed_at TEXT NOT NULL,
    PRIMARY KEY (participant_id, scoring_run_id),
    FOREIGN KEY (participant_id) REFERENCES participants (participant_id),
    FOREIGN KEY (scoring_run_id) REFERENCES scoring_runs (scoring_run_id)
);

CREATE INDEX IF NOT EXISTS idx_participant_relationship_scores_participant_current
    ON participant_relationship_scores (participant_id, is_current);
CREATE INDEX IF NOT EXISTS idx_participant_relationship_scores_scoring_run
    ON participant_relationship_scores (scoring_run_id);

CREATE TABLE IF NOT EXISTS signal_scores (
    thread_id TEXT NOT NULL,
    scoring_run_id TEXT NOT NULL,
    relationship_score REAL NOT NULL,
    urgency_score REAL NOT NULL,
    actionability_score REAL NOT NULL,
    priority_score REAL NOT NULL,
    explanation_json TEXT,
    is_current INTEGER NOT NULL CHECK (is_current IN (0, 1)),
    computed_at TEXT NOT NULL,
    PRIMARY KEY (thread_id, scoring_run_id),
    FOREIGN KEY (thread_id) REFERENCES threads (thread_id),
    FOREIGN KEY (scoring_run_id) REFERENCES scoring_runs (scoring_run_id)
);

CREATE INDEX IF NOT EXISTS idx_signal_scores_thread_current
    ON signal_scores (thread_id, is_current);
CREATE INDEX IF NOT EXISTS idx_signal_scores_current_priority_desc
    ON signal_scores (is_current, priority_score DESC);
CREATE INDEX IF NOT EXISTS idx_signal_scores_scoring_run
    ON signal_scores (scoring_run_id);

CREATE TABLE IF NOT EXISTS briefing_entries (
    briefing_entry_id TEXT PRIMARY KEY,
    scoring_run_id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    headline TEXT NOT NULL,
    why_it_matters TEXT NOT NULL,
    suggested_next_action TEXT,
    confidence REAL NOT NULL,
    generated_by TEXT NOT NULL,
    rank_position INTEGER NOT NULL,
    is_current INTEGER NOT NULL CHECK (is_current IN (0, 1)),
    generated_at TEXT NOT NULL,
    FOREIGN KEY (scoring_run_id) REFERENCES scoring_runs (scoring_run_id),
    FOREIGN KEY (thread_id) REFERENCES threads (thread_id)
);

CREATE INDEX IF NOT EXISTS idx_briefing_entries_current_rank_position
    ON briefing_entries (is_current, rank_position);
CREATE INDEX IF NOT EXISTS idx_briefing_entries_thread_id
    ON briefing_entries (thread_id);
CREATE INDEX IF NOT EXISTS idx_briefing_entries_scoring_run
    ON briefing_entries (scoring_run_id);
