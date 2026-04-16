use std::collections::{BTreeSet, HashMap};

use briefly_contracts::{
    FocusDashboardResponse, ImportBatchStatus, Participant, RankedThreadCard,
    ScoreExplanationPayload, ScoringRunStatus, ThreadComponentScores,
};
use briefly_store::Store;
use chrono::{DateTime, Duration, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

const SCORING_VERSION: &str = "phase1_v1";
const ASK_PHRASES: [&str; 8] = [
    "can you",
    "could you",
    "please",
    "need you to",
    "let me know",
    "what do you think",
    "would you",
    "able to",
];
const FOLLOW_UP_PHRASES: [&str; 5] = [
    "following up",
    "checking in",
    "bumping this",
    "circling back",
    "gentle reminder",
];
const DEADLINE_PHRASES: [&str; 9] = [
    "by friday",
    "tomorrow",
    "next week",
    "before",
    "deadline",
    "eta",
    "schedule",
    "meeting",
    "calendar",
];
const ATTACHMENT_PHRASES: [&str; 6] = ["attached", "deck", "doc", "proposal", "contract", "resume"];
const BULK_PATTERNS: [&str; 8] = [
    "receipt",
    "invoice paid",
    "password reset",
    "verify your email",
    "newsletter",
    "unsubscribe",
    "no-reply",
    "notifications",
];

pub fn bootstrap_scope() -> &'static str {
    "deterministic ranking, explanation payloads, and rescoring boundaries"
}

pub fn run_scoring(
    store: &mut Store,
    trigger_import_batch_id: Option<&str>,
) -> rusqlite::Result<()> {
    let snapshot = canonical_snapshot(store.connection())?;
    let scored_threads = build_ranked_threads(&snapshot);
    persist_scoring_run(store, trigger_import_batch_id, &snapshot, &scored_threads)?;
    Ok(())
}

pub fn load_focus_dashboard(store: &Store) -> rusqlite::Result<FocusDashboardResponse> {
    let connection = store.connection();
    let has_imported_mailbox = connection
        .query_row("SELECT EXISTS(SELECT 1 FROM import_batches)", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|value| value != 0)?;

    let last_import_status = connection
        .query_row(
            "SELECT status FROM import_batches ORDER BY started_at DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|status| parse_import_status(&status));

    let last_scoring_status = connection
        .query_row(
            "SELECT status FROM scoring_runs ORDER BY started_at DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .and_then(|status| parse_scoring_status(&status));

    let generated_at = connection.query_row(
        "SELECT MAX(computed_at) FROM signal_scores WHERE is_current = 1",
        [],
        |row| row.get::<_, Option<String>>(0),
    )?;

    let mut statement = connection.prepare(
        "SELECT
            scores.thread_id,
            threads.canonical_subject,
            threads.latest_message_at,
            threads.message_count,
            messages.body_preview,
            scores.explanation_json
        FROM signal_scores scores
        INNER JOIN threads ON threads.thread_id = scores.thread_id
        LEFT JOIN messages ON messages.message_id = (
            SELECT inner_messages.message_id
            FROM messages inner_messages
            WHERE inner_messages.thread_id = scores.thread_id
            ORDER BY inner_messages.sent_at DESC, inner_messages.created_at DESC
            LIMIT 1
        )
        WHERE scores.is_current = 1
        ORDER BY scores.priority_score DESC, threads.latest_message_at DESC",
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let participants = participants_by_thread(connection)?;
    let mut threads = Vec::with_capacity(rows.len());

    for (
        thread_id,
        canonical_subject,
        latest_message_at,
        message_count,
        latest_message_preview,
        explanation_json,
    ) in rows
    {
        let explanation: ScoreExplanationPayload =
            serde_json::from_str(&explanation_json).unwrap_or_else(|_| empty_explanation());

        threads.push(RankedThreadCard {
            thread_id: thread_id.clone(),
            canonical_subject,
            latest_message_at,
            latest_message_preview,
            message_count: message_count as usize,
            participants: participants.get(&thread_id).cloned().unwrap_or_default(),
            scores: explanation.component_scores.clone(),
            explanation,
        });
    }

    Ok(FocusDashboardResponse {
        generated_at,
        has_imported_mailbox,
        last_import_status,
        last_scoring_status,
        threads,
    })
}

#[derive(Clone)]
struct CanonicalSnapshot {
    threads: Vec<ThreadData>,
    participant_message_counts: HashMap<String, usize>,
    participant_last_seen_at: HashMap<String, Option<String>>,
}

#[derive(Clone)]
struct ThreadData {
    thread_id: String,
    canonical_subject: Option<String>,
    latest_message_at: Option<String>,
    message_count: usize,
    participants: Vec<Participant>,
    messages: Vec<MessageData>,
}

#[derive(Clone)]
struct MessageData {
    sender_participant_id: Option<String>,
    sender_email: Option<String>,
    subject: Option<String>,
    body_text: Option<String>,
    body_preview: Option<String>,
    sent_at: Option<String>,
    recipient_count: usize,
}

fn canonical_snapshot(connection: &Connection) -> rusqlite::Result<CanonicalSnapshot> {
    let participant_message_counts = participant_message_counts(connection)?;
    let participant_last_seen_at = participant_last_seen(connection)?;
    let participants = participants_by_thread(connection)?;
    let mut threads = thread_rows(connection)?;
    let messages = messages_by_thread(connection)?;

    for thread in &mut threads {
        thread.participants = participants
            .get(&thread.thread_id)
            .cloned()
            .unwrap_or_default();
        thread.messages = messages.get(&thread.thread_id).cloned().unwrap_or_default();
    }

    Ok(CanonicalSnapshot {
        threads,
        participant_message_counts,
        participant_last_seen_at,
    })
}

fn participant_message_counts(connection: &Connection) -> rusqlite::Result<HashMap<String, usize>> {
    let mut statement = connection.prepare(
        "SELECT participant_id, COUNT(*) FROM (
            SELECT sender_participant_id AS participant_id FROM messages WHERE sender_participant_id IS NOT NULL
            UNION ALL
            SELECT participant_id FROM message_participants
        )
        GROUP BY participant_id",
    )?;

    let mut output = HashMap::new();
    for row in statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })? {
        let (participant_id, count) = row?;
        output.insert(participant_id, count as usize);
    }

    Ok(output)
}

fn participant_last_seen(
    connection: &Connection,
) -> rusqlite::Result<HashMap<String, Option<String>>> {
    let mut statement =
        connection.prepare("SELECT participant_id, last_seen_at FROM participants")?;
    let mut output = HashMap::new();
    for row in statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })? {
        let (participant_id, last_seen_at) = row?;
        output.insert(participant_id, last_seen_at);
    }
    Ok(output)
}

fn thread_rows(connection: &Connection) -> rusqlite::Result<Vec<ThreadData>> {
    let mut statement = connection.prepare(
        "SELECT thread_id, canonical_subject, latest_message_at, message_count
        FROM threads
        ORDER BY latest_message_at DESC",
    )?;

    let rows = statement
        .query_map([], |row| {
            Ok(ThreadData {
                thread_id: row.get(0)?,
                canonical_subject: row.get(1)?,
                latest_message_at: row.get(2)?,
                message_count: row.get::<_, i64>(3)? as usize,
                participants: Vec::new(),
                messages: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(rows)
}

fn messages_by_thread(
    connection: &Connection,
) -> rusqlite::Result<HashMap<String, Vec<MessageData>>> {
    let mut statement = connection.prepare(
        "SELECT
            messages.thread_id,
            messages.sender_participant_id,
            participants.normalized_email,
            messages.subject,
            messages.body_text,
            messages.body_preview,
            messages.sent_at,
            (
                SELECT COUNT(*)
                FROM message_participants
                WHERE message_participants.message_id = messages.message_id
            ) AS recipient_count
        FROM messages
        LEFT JOIN participants ON participants.participant_id = messages.sender_participant_id
        ORDER BY messages.thread_id, messages.sent_at DESC, messages.created_at DESC",
    )?;

    let mut output = HashMap::<String, Vec<MessageData>>::new();
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            MessageData {
                sender_participant_id: row.get(1)?,
                sender_email: row.get(2)?,
                subject: row.get(3)?,
                body_text: row.get(4)?,
                body_preview: row.get(5)?,
                sent_at: row.get(6)?,
                recipient_count: row.get::<_, i64>(7)? as usize,
            },
        ))
    })? {
        let (thread_id, message) = row?;
        output.entry(thread_id).or_default().push(message);
    }

    Ok(output)
}

fn participants_by_thread(
    connection: &Connection,
) -> rusqlite::Result<HashMap<String, Vec<Participant>>> {
    let mut statement = connection.prepare(
        "SELECT DISTINCT
            messages.thread_id,
            participants.participant_id,
            participants.normalized_email,
            participants.display_name
        FROM messages
        INNER JOIN participants ON participants.participant_id = messages.sender_participant_id
        UNION
        SELECT DISTINCT
            messages.thread_id,
            participants.participant_id,
            participants.normalized_email,
            participants.display_name
        FROM messages
        INNER JOIN message_participants ON message_participants.message_id = messages.message_id
        INNER JOIN participants ON participants.participant_id = message_participants.participant_id
        ORDER BY 1, 3",
    )?;

    let mut output = HashMap::<String, Vec<Participant>>::new();
    for row in statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            Participant {
                participant_id: row.get(1)?,
                email: row.get(2)?,
                display_name: row.get(3)?,
            },
        ))
    })? {
        let (thread_id, participant) = row?;
        output.entry(thread_id).or_default().push(participant);
    }

    for participants in output.values_mut() {
        participants.sort_by(|left, right| left.email.cmp(&right.email));
        participants.dedup_by(|left, right| left.participant_id == right.participant_id);
    }

    Ok(output)
}

fn build_ranked_threads(snapshot: &CanonicalSnapshot) -> Vec<RankedThreadCard> {
    let mut threads = snapshot
        .threads
        .iter()
        .map(|thread| score_thread(snapshot, thread))
        .collect::<Vec<_>>();

    threads.sort_by(|left, right| {
        right
            .scores
            .priority_score
            .partial_cmp(&left.scores.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.latest_message_at.cmp(&left.latest_message_at))
    });

    threads
}

fn score_thread(snapshot: &CanonicalSnapshot, thread: &ThreadData) -> RankedThreadCard {
    let latest_messages = thread.messages.iter().take(2).cloned().collect::<Vec<_>>();
    let searchable_text = latest_messages
        .iter()
        .flat_map(|message| [message.subject.as_deref(), message.body_text.as_deref()])
        .flatten()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();

    let distinct_senders = thread
        .messages
        .iter()
        .filter_map(|message| message.sender_participant_id.clone())
        .collect::<BTreeSet<_>>();
    let reciprocity = if distinct_senders.len() >= 2 {
        1.0
    } else {
        0.0
    };
    let directness = match thread
        .messages
        .iter()
        .map(|message| message.recipient_count)
        .min()
        .unwrap_or(0)
    {
        0..=2 => 1.0,
        3..=5 => 0.5,
        _ => 0.0,
    };
    let participant_familiarity = thread
        .participants
        .iter()
        .map(|participant| {
            let volume = (snapshot
                .participant_message_counts
                .get(&participant.participant_id)
                .copied()
                .unwrap_or(0) as f64
                / 12.0)
                .min(1.0);
            let recency = recency_bucket(
                snapshot
                    .participant_last_seen_at
                    .get(&participant.participant_id)
                    .and_then(|value| value.as_deref()),
            );
            (volume * 0.35) + (reciprocity * 0.25) + (directness * 0.20) + (recency * 0.20)
        })
        .fold(0.0_f64, f64::max);
    let relationship_score = round4(
        (participant_familiarity
            + if (3..=5).contains(&thread.participants.len()) {
                0.05
            } else {
                0.0
            })
        .min(1.0),
    );

    let direct_ask = contains_phrase(&searchable_text, &ASK_PHRASES);
    let follow_up = contains_phrase(&searchable_text, &FOLLOW_UP_PHRASES);
    let deadline = contains_phrase(&searchable_text, &DEADLINE_PHRASES);
    let attachment = contains_phrase(&searchable_text, &ATTACHMENT_PHRASES);
    let unanswered_inbound = distinct_senders.len() >= 2
        && thread.messages.len() >= 2
        && thread.messages[0].sender_participant_id != thread.messages[1].sender_participant_id;
    let actionability_score = round4(
        (if direct_ask { 0.35 } else { 0.0 })
            + (if follow_up { 0.20 } else { 0.0 })
            + (if deadline { 0.20 } else { 0.0 })
            + (if unanswered_inbound { 0.15 } else { 0.0 })
            + (if attachment { 0.10 } else { 0.0 }),
    );

    let rapid_back_and_forth = recent_message_count(&thread.messages, 48);
    let rapid_back_and_forth_score = match rapid_back_and_forth {
        3.. => 1.0,
        2 => 0.5,
        _ => 0.0,
    };
    let follow_up_after_silence = follow_up
        && thread.messages.len() >= 2
        && age_between(
            thread.messages[0].sent_at.as_deref(),
            thread.messages[1].sent_at.as_deref(),
        )
        .is_some_and(|gap| gap > Duration::days(3));
    let same_day_freshness = age_from_now(thread.latest_message_at.as_deref())
        .is_some_and(|age| age <= Duration::hours(24));
    let urgency_score = round4(
        (if deadline { 0.40 } else { 0.0 })
            + (rapid_back_and_forth_score * 0.30)
            + (if follow_up_after_silence { 0.20 } else { 0.0 })
            + (if same_day_freshness { 0.10 } else { 0.0 }),
    );

    let recency_score = round4(recency_bucket(thread.latest_message_at.as_deref()));
    let thread_activity_bonus = if thread.message_count >= 4 && thread.participants.len() >= 2 {
        0.05
    } else {
        0.0
    };
    let bulk_penalty = bulk_penalty(&searchable_text, &latest_messages);
    let priority_score = round4(
        ((relationship_score * 0.40)
            + (actionability_score * 0.30)
            + (urgency_score * 0.20)
            + (recency_score * 0.10)
            + thread_activity_bonus
            - bulk_penalty)
            .clamp(0.0, 1.0),
    );

    let mut top_reasons = Vec::new();
    let mut matched_signals = Vec::new();
    if relationship_score >= 0.70 {
        top_reasons.push("Strong relationship with sender".to_string());
        matched_signals.push("participant_familiarity_high".to_string());
    }
    if direct_ask {
        top_reasons.push("Direct ask detected".to_string());
        matched_signals.push("ask_language_present".to_string());
    }
    if rapid_back_and_forth_score >= 0.5 {
        top_reasons.push("Recent reply activity".to_string());
        matched_signals.push("recent_back_and_forth".to_string());
    }
    if unanswered_inbound {
        top_reasons.push("Awaiting your response".to_string());
        matched_signals.push("awaiting_response".to_string());
    }
    if deadline {
        top_reasons.push("Time-sensitive language detected".to_string());
        matched_signals.push("deadline_language_present".to_string());
    }

    let mut applied_penalties = Vec::new();
    if bulk_penalty > 0.0 {
        top_reasons.push("Likely bulk or automated mail".to_string());
        applied_penalties.push(if bulk_penalty >= 0.25 {
            "bulk_noise_high".to_string()
        } else {
            "bulk_noise_partial".to_string()
        });
    }

    if top_reasons.is_empty() {
        top_reasons.push("Recent thread activity".to_string());
        matched_signals.push("recency_support".to_string());
    }
    top_reasons.truncate(3);

    let scores = ThreadComponentScores {
        relationship_score,
        actionability_score,
        urgency_score,
        recency_score,
        priority_score,
    };
    let explanation = ScoreExplanationPayload {
        version: SCORING_VERSION.to_string(),
        top_reasons,
        component_scores: scores.clone(),
        matched_signals,
        applied_penalties,
    };

    RankedThreadCard {
        thread_id: thread.thread_id.clone(),
        canonical_subject: thread.canonical_subject.clone(),
        latest_message_at: thread.latest_message_at.clone(),
        latest_message_preview: thread
            .messages
            .first()
            .and_then(|message| message.body_preview.clone()),
        message_count: thread.message_count,
        participants: thread.participants.clone(),
        scores,
        explanation,
    }
}

fn persist_scoring_run(
    store: &mut Store,
    trigger_import_batch_id: Option<&str>,
    snapshot: &CanonicalSnapshot,
    scored_threads: &[RankedThreadCard],
) -> rusqlite::Result<()> {
    let now = Utc::now().to_rfc3339();
    let scoring_run_id = prefixed_digest(
        "scr",
        &format!(
            "{SCORING_VERSION}:{now}:{}",
            trigger_import_batch_id.unwrap_or("manual")
        ),
    );
    let tx = store.connection_mut().transaction()?;

    tx.execute(
        "UPDATE participant_relationship_scores SET is_current = 0 WHERE is_current = 1",
        [],
    )?;
    tx.execute(
        "UPDATE signal_scores SET is_current = 0 WHERE is_current = 1",
        [],
    )?;
    tx.execute(
        "UPDATE briefing_entries SET is_current = 0 WHERE is_current = 1",
        [],
    )?;

    tx.execute(
        "INSERT INTO scoring_runs (
            scoring_run_id,
            trigger_import_batch_id,
            scoring_version,
            status,
            started_at,
            completed_at,
            notes
        ) VALUES (?1, ?2, ?3, 'completed', ?4, ?4, ?5)",
        params![
            scoring_run_id,
            trigger_import_batch_id,
            SCORING_VERSION,
            now,
            format!(
                "Ranked {} thread(s) for focus dashboard.",
                scored_threads.len()
            ),
        ],
    )?;

    for thread in &snapshot.threads {
        for participant in &thread.participants {
            let relationship_score = snapshot
                .participant_message_counts
                .get(&participant.participant_id)
                .copied()
                .unwrap_or(0) as f64
                / 12.0;
            tx.execute(
                "INSERT INTO participant_relationship_scores (
                    participant_id,
                    scoring_run_id,
                    relationship_score,
                    interaction_count,
                    last_interaction_at,
                    is_current,
                    computed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)
                ON CONFLICT(participant_id, scoring_run_id) DO NOTHING",
                params![
                    participant.participant_id,
                    scoring_run_id,
                    round4(relationship_score.min(1.0)),
                    snapshot
                        .participant_message_counts
                        .get(&participant.participant_id)
                        .copied()
                        .unwrap_or(0) as i64,
                    snapshot
                        .participant_last_seen_at
                        .get(&participant.participant_id)
                        .cloned()
                        .flatten(),
                    now,
                ],
            )?;
        }
    }

    for thread in scored_threads {
        tx.execute(
            "INSERT INTO signal_scores (
                thread_id,
                scoring_run_id,
                relationship_score,
                urgency_score,
                actionability_score,
                priority_score,
                explanation_json,
                is_current,
                computed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8)",
            params![
                thread.thread_id,
                scoring_run_id,
                thread.scores.relationship_score,
                thread.scores.urgency_score,
                thread.scores.actionability_score,
                thread.scores.priority_score,
                serde_json::to_string(&thread.explanation).unwrap_or_default(),
                now,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

fn recency_bucket(value: Option<&str>) -> f64 {
    let Some(age) = age_from_now(value) else {
        return 0.05;
    };

    if age <= Duration::hours(24) {
        1.0
    } else if age <= Duration::days(3) {
        0.8
    } else if age <= Duration::days(7) {
        0.6
    } else if age <= Duration::days(14) {
        0.35
    } else if age <= Duration::days(30) {
        0.15
    } else {
        0.05
    }
}

fn age_from_now(value: Option<&str>) -> Option<Duration> {
    let timestamp = parse_timestamp(value?)?;
    Some(Utc::now() - timestamp)
}

fn age_between(later: Option<&str>, earlier: Option<&str>) -> Option<Duration> {
    Some(parse_timestamp(later?)? - parse_timestamp(earlier?)?)
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

fn recent_message_count(messages: &[MessageData], hours: i64) -> usize {
    let Some(latest) = messages
        .first()
        .and_then(|message| message.sent_at.as_deref())
        .and_then(parse_timestamp)
    else {
        return 0;
    };
    messages
        .iter()
        .filter_map(|message| message.sent_at.as_deref())
        .filter_map(parse_timestamp)
        .filter(|sent_at| latest.signed_duration_since(*sent_at) <= Duration::hours(hours))
        .count()
}

fn contains_phrase(haystack: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| haystack.contains(phrase))
}

fn bulk_penalty(searchable_text: &str, latest_messages: &[MessageData]) -> f64 {
    let matches_bulk_pattern = BULK_PATTERNS
        .iter()
        .any(|pattern| searchable_text.contains(pattern));
    let automated_sender = latest_messages.iter().any(|message| {
        message.sender_email.as_deref().is_some_and(|email| {
            ["no-reply", "noreply", "notifications", "mailer-daemon"]
                .iter()
                .any(|token| email.contains(token))
        })
    });
    let large_recipient_count = latest_messages
        .iter()
        .map(|message| message.recipient_count)
        .max()
        .unwrap_or(0)
        > 10;

    if matches_bulk_pattern || automated_sender {
        0.25
    } else if large_recipient_count {
        0.10
    } else {
        0.0
    }
}

fn parse_import_status(status: &str) -> Option<ImportBatchStatus> {
    match status {
        "completed" => Some(ImportBatchStatus::Completed),
        "partial" => Some(ImportBatchStatus::Partial),
        "failed" => Some(ImportBatchStatus::Failed),
        _ => None,
    }
}

fn parse_scoring_status(status: &str) -> Option<ScoringRunStatus> {
    match status {
        "running" => Some(ScoringRunStatus::Running),
        "completed" => Some(ScoringRunStatus::Completed),
        "failed" => Some(ScoringRunStatus::Failed),
        _ => None,
    }
}

fn empty_explanation() -> ScoreExplanationPayload {
    ScoreExplanationPayload {
        version: SCORING_VERSION.to_string(),
        top_reasons: vec![],
        component_scores: ThreadComponentScores {
            relationship_score: 0.0,
            actionability_score: 0.0,
            urgency_score: 0.0,
            recency_score: 0.0,
            priority_score: 0.0,
        },
        matched_signals: vec![],
        applied_penalties: vec![],
    }
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

fn prefixed_digest(prefix: &str, value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{prefix}_{}", &hex[..16])
}

#[cfg(test)]
mod tests {
    use super::*;
    use briefly_contracts::{
        ImportBatchOutput, ImportBatchStatus, NormalizedMessage, RejectedMessage, Thread,
    };
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct DashboardSeed {
        threads: Vec<DashboardSeedThread>,
    }

    #[derive(Debug, Deserialize)]
    struct DashboardSeedThread {
        thread_id: String,
        subject: String,
        priority_score: f64,
        top_reasons: Vec<String>,
    }

    fn seeded_output() -> ImportBatchOutput {
        ImportBatchOutput {
            import_batch_id: "bat_test".to_string(),
            source_path: "/tmp/dashboard.mbox".to_string(),
            source_fingerprint: "src_test".to_string(),
            imported_at: "2026-04-15T12:00:00Z".to_string(),
            parser_version: "briefly-ingest/0.1.0".to_string(),
            status: ImportBatchStatus::Completed,
            message_count_seen: 3,
            accepted_messages: vec![
                NormalizedMessage {
                    message_key: "msg_a".to_string(),
                    raw_message_id: Some("a@example.com".to_string()),
                    thread_id: "thr_focus".to_string(),
                    subject: Some("Can you review the investor update?".to_string()),
                    canonical_subject: Some("can you review the investor update?".to_string()),
                    sender_participant_id: "par_founder".to_string(),
                    sender: Participant {
                        participant_id: "par_founder".to_string(),
                        email: "founder@example.com".to_string(),
                        display_name: Some("Founder".to_string()),
                    },
                    to: vec![Participant {
                        participant_id: "par_operator".to_string(),
                        email: "operator@example.com".to_string(),
                        display_name: Some("Operator".to_string()),
                    }],
                    cc: vec![],
                    bcc: vec![],
                    reply_to: vec![],
                    sent_at: Some(Utc::now().to_rfc3339()),
                    body_text: Some("Can you review the attached update by tomorrow?".to_string()),
                    body_preview: Some(
                        "Can you review the attached update by tomorrow?".to_string(),
                    ),
                    body_text_digest: Some("digest_a".to_string()),
                    has_html_body: false,
                },
                NormalizedMessage {
                    message_key: "msg_b".to_string(),
                    raw_message_id: Some("b@example.com".to_string()),
                    thread_id: "thr_bulk".to_string(),
                    subject: Some("Newsletter receipt".to_string()),
                    canonical_subject: Some("newsletter receipt".to_string()),
                    sender_participant_id: "par_notice".to_string(),
                    sender: Participant {
                        participant_id: "par_notice".to_string(),
                        email: "notifications@example.com".to_string(),
                        display_name: Some("Notifications".to_string()),
                    },
                    to: vec![Participant {
                        participant_id: "par_operator".to_string(),
                        email: "operator@example.com".to_string(),
                        display_name: Some("Operator".to_string()),
                    }],
                    cc: vec![],
                    bcc: vec![],
                    reply_to: vec![],
                    sent_at: Some(Utc::now().to_rfc3339()),
                    body_text: Some("Unsubscribe from this newsletter".to_string()),
                    body_preview: Some("Unsubscribe from this newsletter".to_string()),
                    body_text_digest: Some("digest_b".to_string()),
                    has_html_body: false,
                },
            ],
            rejected_messages: vec![RejectedMessage {
                source_index: 2,
                reason: "partial parse".to_string(),
            }],
            participants: vec![
                Participant {
                    participant_id: "par_founder".to_string(),
                    email: "founder@example.com".to_string(),
                    display_name: Some("Founder".to_string()),
                },
                Participant {
                    participant_id: "par_operator".to_string(),
                    email: "operator@example.com".to_string(),
                    display_name: Some("Operator".to_string()),
                },
                Participant {
                    participant_id: "par_notice".to_string(),
                    email: "notifications@example.com".to_string(),
                    display_name: Some("Notifications".to_string()),
                },
            ],
            threads: vec![
                Thread {
                    thread_id: "thr_focus".to_string(),
                    canonical_subject: Some("can you review the investor update?".to_string()),
                    root_message_key: "msg_a".to_string(),
                    latest_message_at: Some(Utc::now().to_rfc3339()),
                    message_count: 1,
                },
                Thread {
                    thread_id: "thr_bulk".to_string(),
                    canonical_subject: Some("newsletter receipt".to_string()),
                    root_message_key: "msg_b".to_string(),
                    latest_message_at: Some(Utc::now().to_rfc3339()),
                    message_count: 1,
                },
            ],
        }
    }

    #[test]
    fn scoring_ranks_human_action_thread_above_bulk_thread() {
        let mut store = Store::open_in_memory().expect("store should initialize");
        let output = seeded_output();
        let report = store
            .persist_import_batch(&output)
            .expect("import should persist");

        run_scoring(&mut store, Some(report.import_batch_id.as_str()))
            .expect("scoring should persist");
        let dashboard = load_focus_dashboard(&store).expect("dashboard should load");

        assert_eq!(dashboard.threads.len(), 2);
        assert_eq!(dashboard.threads[0].thread_id, "thr_focus");
        assert!(dashboard.threads[0]
            .explanation
            .top_reasons
            .contains(&"Direct ask detected".to_string()));
        assert_eq!(dashboard.threads[1].thread_id, "thr_bulk");
        assert!(dashboard.threads[1].explanation.applied_penalties.len() >= 1);
    }

    #[test]
    fn dashboard_seed_fixture_keeps_visual_artifact_shape_stable() {
        let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/ui/dashboard-seed.json");
        let fixture = std::fs::read_to_string(fixture_path).expect("fixture should read");
        let seed: DashboardSeed = serde_json::from_str(&fixture).expect("fixture should parse");

        assert_eq!(seed.threads.len(), 1);
        assert_eq!(seed.threads[0].thread_id, "thr_01jbootstrap");
        assert_eq!(
            seed.threads[0].subject,
            "Can you review the investor update?"
        );
        assert!(seed.threads[0].priority_score > 0.7);
        assert_eq!(seed.threads[0].top_reasons.len(), 2);
    }
}
