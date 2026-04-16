use briefly_contracts::{ImportBatchOutput, ImportBatchStatus, NormalizedMessage, Participant};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;

const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_initial_schema.sql");
const SCHEMA_VERSION: i32 = 1;

pub fn bootstrap_scope() -> &'static str {
    "sqlite access, migrations, and persistence repositories"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: [Migration; 1] = [Migration {
    version: SCHEMA_VERSION,
    name: "0001_initial_schema",
    sql: INITIAL_SCHEMA,
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryBoundary {
    CanonicalMail,
    DerivedReadModels,
}

impl RepositoryBoundary {
    pub fn description(self) -> &'static str {
        match self {
            Self::CanonicalMail => {
                "Owns import provenance plus canonical participants, threads, messages, and message links."
            }
            Self::DerivedReadModels => {
                "Owns scoring runs, relationship snapshots, thread scores, and briefing snapshots."
            }
        }
    }
}

pub const REPOSITORY_BOUNDARIES: [RepositoryBoundary; 2] = [
    RepositoryBoundary::CanonicalMail,
    RepositoryBoundary::DerivedReadModels,
];

pub struct Store {
    connection: Connection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistImportReport {
    pub import_batch_id: String,
    pub linked_messages: usize,
    pub parse_error_count: usize,
}

impl Store {
    pub fn open_path(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        Self::initialize(connection)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let connection = Connection::open_in_memory()?;
        Self::initialize(connection)
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn persist_import_batch(
        &mut self,
        output: &ImportBatchOutput,
    ) -> rusqlite::Result<PersistImportReport> {
        let tx = self.connection.transaction()?;
        let report = persist_import_batch(&tx, output)?;
        tx.commit()?;
        Ok(report)
    }

    fn initialize(mut connection: Connection) -> rusqlite::Result<Self> {
        connection.pragma_update(None, "foreign_keys", 1)?;
        apply_migrations(&mut connection)?;
        Ok(Self { connection })
    }
}

pub fn apply_migrations(connection: &mut Connection) -> rusqlite::Result<()> {
    let tx = connection.transaction()?;
    let current_version: i32 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;

    for migration in MIGRATIONS
        .iter()
        .filter(|migration| migration.version > current_version)
    {
        tx.execute_batch(migration.sql)?;
        tx.pragma_update(None, "user_version", migration.version)?;
    }

    tx.commit()?;
    Ok(())
}

pub trait CanonicalRepository {
    fn import_batches_table(&self) -> &'static str {
        "import_batches"
    }

    fn message_sources_table(&self) -> &'static str {
        "message_sources"
    }

    fn participants_table(&self) -> &'static str {
        "participants"
    }

    fn threads_table(&self) -> &'static str {
        "threads"
    }

    fn messages_table(&self) -> &'static str {
        "messages"
    }

    fn message_participants_table(&self) -> &'static str {
        "message_participants"
    }
}

pub trait DerivedRepository {
    fn scoring_runs_table(&self) -> &'static str {
        "scoring_runs"
    }

    fn participant_relationship_scores_table(&self) -> &'static str {
        "participant_relationship_scores"
    }

    fn signal_scores_table(&self) -> &'static str {
        "signal_scores"
    }

    fn briefing_entries_table(&self) -> &'static str {
        "briefing_entries"
    }
}

pub struct SqliteRepositories;

impl CanonicalRepository for SqliteRepositories {}
impl DerivedRepository for SqliteRepositories {}

fn persist_import_batch(
    tx: &Transaction<'_>,
    output: &ImportBatchOutput,
) -> rusqlite::Result<PersistImportReport> {
    let stored_import_batch_id = prefixed_digest("imp", &format!("{}:{}", output.import_batch_id, output.imported_at));
    let source_filename = Path::new(&output.source_path)
        .file_name()
        .unwrap_or_else(|| OsStr::new(""))
        .to_string_lossy()
        .into_owned();
    let file_size_bytes = std::fs::metadata(&output.source_path)
        .map(|metadata| metadata.len() as i64)
        .unwrap_or(0);
    let notes = build_import_notes(output);
    let parse_error_count = output.rejected_messages.len() as i64;
    let message_count_linked = output.accepted_messages.len() as i64;

    tx.execute(
        "INSERT INTO import_batches (
            import_batch_id,
            source_path,
            source_filename,
            source_sha256,
            file_size_bytes,
            parser_version,
            status,
            started_at,
            completed_at,
            message_count_seen,
            message_count_linked,
            parse_error_count,
            notes
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            stored_import_batch_id,
            output.source_path,
            source_filename,
            output.source_fingerprint,
            file_size_bytes,
            output.parser_version,
            import_status_as_str(&output.status),
            output.imported_at,
            output.imported_at,
            output.message_count_seen as i64,
            message_count_linked,
            parse_error_count,
            notes,
        ],
    )?;

    for participant in &output.participants {
        upsert_participant(tx, participant, output.imported_at.as_str())?;
    }

    for thread in &output.threads {
        let participant_count = participant_ids_for_thread(&output.accepted_messages, &thread.thread_id).len() as i64;
        tx.execute(
            "INSERT INTO threads (
                thread_id,
                thread_key,
                canonical_subject,
                latest_message_at,
                message_count,
                participant_count,
                threading_version,
                created_at,
                updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            ON CONFLICT(thread_key) DO UPDATE SET
                canonical_subject = COALESCE(excluded.canonical_subject, threads.canonical_subject),
                latest_message_at = CASE
                    WHEN threads.latest_message_at IS NULL THEN excluded.latest_message_at
                    WHEN excluded.latest_message_at IS NULL THEN threads.latest_message_at
                    WHEN excluded.latest_message_at > threads.latest_message_at THEN excluded.latest_message_at
                    ELSE threads.latest_message_at
                END,
                message_count = MAX(threads.message_count, excluded.message_count),
                participant_count = MAX(threads.participant_count, excluded.participant_count),
                updated_at = excluded.updated_at",
            params![
                thread.thread_id,
                thread.thread_id,
                thread.canonical_subject,
                thread.latest_message_at,
                thread.message_count as i64,
                participant_count,
                "briefly-ingest/0.1.0",
                output.imported_at,
                output.imported_at,
            ],
        )?;
    }

    let accepted_positions = accepted_source_positions(output);

    for (accepted_index, message) in output.accepted_messages.iter().enumerate() {
        let message_id = ensure_message(tx, output, &stored_import_batch_id, message)?;
        sync_message_participants(tx, &message_id, message)?;
        let source_position = accepted_positions[accepted_index];
        insert_message_source(
            tx,
            &stored_import_batch_id,
            Some(&message_id),
            source_position,
            "parsed",
            None,
            &format!("accepted:{}", message.message_key),
            output.imported_at.as_str(),
        )?;
    }

    for rejected in &output.rejected_messages {
        insert_message_source(
            tx,
            &stored_import_batch_id,
            None,
            rejected.source_index,
            "failed",
            Some(rejected.reason.as_str()),
            &format!("rejected:{}:{}", rejected.source_index, rejected.reason),
            output.imported_at.as_str(),
        )?;
    }

    Ok(PersistImportReport {
        import_batch_id: stored_import_batch_id,
        linked_messages: output.accepted_messages.len(),
        parse_error_count: output.rejected_messages.len(),
    })
}

fn upsert_participant(
    tx: &Transaction<'_>,
    participant: &Participant,
    imported_at: &str,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT INTO participants (
            participant_id,
            normalized_email,
            display_name,
            organization_hint,
            first_seen_at,
            last_seen_at,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(normalized_email) DO UPDATE SET
            display_name = COALESCE(excluded.display_name, participants.display_name),
            last_seen_at = CASE
                WHEN excluded.last_seen_at > participants.last_seen_at THEN excluded.last_seen_at
                ELSE participants.last_seen_at
            END,
            updated_at = excluded.updated_at",
        params![
            participant.participant_id,
            participant.email,
            participant.display_name,
            organization_hint(&participant.email),
            imported_at,
            imported_at,
            imported_at,
            imported_at,
        ],
    )?;

    Ok(())
}

fn ensure_message(
    tx: &Transaction<'_>,
    output: &ImportBatchOutput,
    stored_import_batch_id: &str,
    message: &NormalizedMessage,
) -> rusqlite::Result<String> {
    let existing_message_id = tx
        .query_row(
            "SELECT message_id FROM messages WHERE canonical_message_key = ?1",
            params![message.message_key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    let message_id = existing_message_id
        .unwrap_or_else(|| prefixed_digest("mid", &message.message_key));

    tx.execute(
        "INSERT INTO messages (
            message_id,
            canonical_message_key,
            internet_message_id,
            thread_id,
            subject,
            normalized_subject,
            sent_at,
            sender_participant_id,
            body_text,
            body_preview,
            has_attachments,
            import_first_seen_batch_id,
            created_at,
            updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
        ON CONFLICT(canonical_message_key) DO UPDATE SET
            internet_message_id = COALESCE(messages.internet_message_id, excluded.internet_message_id),
            thread_id = excluded.thread_id,
            subject = COALESCE(messages.subject, excluded.subject),
            normalized_subject = COALESCE(messages.normalized_subject, excluded.normalized_subject),
            sent_at = COALESCE(messages.sent_at, excluded.sent_at),
            sender_participant_id = COALESCE(messages.sender_participant_id, excluded.sender_participant_id),
            body_text = CASE
                WHEN messages.body_text IS NULL THEN excluded.body_text
                WHEN excluded.body_text IS NOT NULL AND LENGTH(excluded.body_text) > LENGTH(messages.body_text) THEN excluded.body_text
                ELSE messages.body_text
            END,
            body_preview = COALESCE(messages.body_preview, excluded.body_preview),
            has_attachments = messages.has_attachments OR excluded.has_attachments,
            updated_at = excluded.updated_at",
        params![
            message_id,
            message.message_key,
            message.raw_message_id,
            message.thread_id,
            message.subject,
            message.canonical_subject,
            message.sent_at,
            message.sender_participant_id,
            message.body_text,
            message.body_preview,
            0,
            stored_import_batch_id,
            output.imported_at,
            output.imported_at,
        ],
    )?;

    Ok(message_id)
}

fn sync_message_participants(
    tx: &Transaction<'_>,
    message_id: &str,
    message: &NormalizedMessage,
) -> rusqlite::Result<()> {
    for (position, participant) in message.to.iter().enumerate() {
        insert_message_participant(tx, message_id, &participant.participant_id, "to", position)?;
    }
    for (position, participant) in message.cc.iter().enumerate() {
        insert_message_participant(tx, message_id, &participant.participant_id, "cc", position)?;
    }
    for (position, participant) in message.bcc.iter().enumerate() {
        insert_message_participant(tx, message_id, &participant.participant_id, "bcc", position)?;
    }
    for (position, participant) in message.reply_to.iter().enumerate() {
        insert_message_participant(tx, message_id, &participant.participant_id, "reply_to", position)?;
    }

    Ok(())
}

fn insert_message_participant(
    tx: &Transaction<'_>,
    message_id: &str,
    participant_id: &str,
    role: &str,
    position: usize,
) -> rusqlite::Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO message_participants (
            message_id,
            participant_id,
            role,
            position
        ) VALUES (?1, ?2, ?3, ?4)",
        params![message_id, participant_id, role, position as i64],
    )?;

    Ok(())
}

fn insert_message_source(
    tx: &Transaction<'_>,
    import_batch_id: &str,
    message_id: Option<&str>,
    source_position: usize,
    parse_status: &str,
    parse_error: Option<&str>,
    raw_seed: &str,
    created_at: &str,
) -> rusqlite::Result<()> {
    let source_record_id = prefixed_digest(
        "src",
        &format!("{import_batch_id}:{source_position}:{parse_status}:{raw_seed}"),
    );

    tx.execute(
        "INSERT INTO message_sources (
            source_record_id,
            import_batch_id,
            message_id,
            mailbox_path,
            source_position,
            raw_message_sha256,
            header_blob,
            parse_status,
            parse_error,
            created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            source_record_id,
            import_batch_id,
            message_id,
            Option::<String>::None,
            source_position as i64,
            hex_digest(raw_seed),
            Option::<String>::None,
            parse_status,
            parse_error,
            created_at,
        ],
    )?;

    Ok(())
}

fn participant_ids_for_thread(messages: &[NormalizedMessage], thread_id: &str) -> BTreeSet<String> {
    let mut participants = BTreeSet::new();

    for message in messages.iter().filter(|message| message.thread_id == thread_id) {
        participants.insert(message.sender_participant_id.clone());
        for participant in &message.to {
            participants.insert(participant.participant_id.clone());
        }
        for participant in &message.cc {
            participants.insert(participant.participant_id.clone());
        }
        for participant in &message.bcc {
            participants.insert(participant.participant_id.clone());
        }
        for participant in &message.reply_to {
            participants.insert(participant.participant_id.clone());
        }
    }

    participants
}

fn accepted_source_positions(output: &ImportBatchOutput) -> Vec<usize> {
    let rejected_positions: BTreeSet<usize> = output
        .rejected_messages
        .iter()
        .map(|message| message.source_index)
        .collect();

    (0..output.message_count_seen)
        .filter(|position| !rejected_positions.contains(position))
        .take(output.accepted_messages.len())
        .collect()
}

fn build_import_notes(output: &ImportBatchOutput) -> Option<String> {
    if output.rejected_messages.is_empty() {
        return None;
    }

    let reasons = output
        .rejected_messages
        .iter()
        .take(3)
        .map(|message| format!("#{} {}", message.source_index, message.reason))
        .collect::<Vec<_>>()
        .join("; ");

    Some(format!(
        "{} rejected message(s): {}",
        output.rejected_messages.len(),
        reasons
    ))
}

fn import_status_as_str(status: &ImportBatchStatus) -> &'static str {
    match status {
        ImportBatchStatus::Completed => "completed",
        ImportBatchStatus::Partial => "partial",
        ImportBatchStatus::Failed => "failed",
    }
}

fn organization_hint(email: &str) -> Option<String> {
    email.split_once('@').map(|(_, domain)| domain.to_string())
}

fn prefixed_digest(prefix: &str, value: &str) -> String {
    format!("{prefix}_{}", &hex_digest(value)[..16])
}

fn hex_digest(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::with_capacity(digest.len() * 2);

    for byte in digest {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }

    output
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("value should be a nybble"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_boundaries_are_split_between_canonical_and_derived_data() {
        assert_eq!(REPOSITORY_BOUNDARIES.len(), 2);
        assert!(REPOSITORY_BOUNDARIES.contains(&RepositoryBoundary::CanonicalMail));
        assert!(REPOSITORY_BOUNDARIES
            .iter()
            .any(|boundary| { boundary.description().contains("scoring runs") }));
    }

    #[test]
    fn sqlite_store_bootstrap_reaches_schema_version() {
        let store = Store::open_in_memory().expect("in-memory store should initialize");
        let version: i32 = store
            .connection()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version should be queryable");

        assert_eq!(version, SCHEMA_VERSION);
    }

    #[test]
    fn accepted_source_positions_skip_rejected_indices() {
        let output = ImportBatchOutput {
            import_batch_id: "bat_123".to_string(),
            source_path: "/tmp/example.mbox".to_string(),
            source_fingerprint: "src_123".to_string(),
            imported_at: "2026-04-15T12:00:00Z".to_string(),
            parser_version: "briefly-ingest/0.1.0".to_string(),
            status: ImportBatchStatus::Partial,
            message_count_seen: 3,
            accepted_messages: vec![],
            rejected_messages: vec![
                briefly_contracts::RejectedMessage {
                    source_index: 1,
                    reason: "bad message".to_string(),
                },
            ],
            participants: vec![],
            threads: vec![],
        };

        assert_eq!(accepted_source_positions(&output), Vec::<usize>::new());
    }
}
