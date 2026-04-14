use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::Path;

use briefly_contracts::{
    ImportBatchOutput, ImportBatchStatus, NormalizedMessage, Participant, RejectedMessage, Thread,
};
use chrono::{DateTime, FixedOffset, Utc};
use sha2::{Digest, Sha256};

const PARSER_VERSION: &str = "briefly-ingest/0.1.0";

pub fn bootstrap_scope() -> &'static str {
    "mailbox parsing, normalization, and import diagnostics"
}

#[derive(Debug)]
pub enum IngestError {
    Io(std::io::Error),
    UnsupportedInput(&'static str),
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "failed to read mailbox: {error}"),
            Self::UnsupportedInput(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for IngestError {}

impl From<std::io::Error> for IngestError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn import_mbox_fixture(path: impl AsRef<Path>) -> Result<ImportBatchOutput, IngestError> {
    let path = path.as_ref();

    if path.is_dir() {
        return Err(IngestError::UnsupportedInput(
            "directories and Maildir layouts are not supported",
        ));
    }

    let source = fs::read_to_string(path)?;
    import_mbox_source(path, &source)
}

pub fn import_mbox_source(
    source_path: impl AsRef<Path>,
    source: &str,
) -> Result<ImportBatchOutput, IngestError> {
    let source_path = source_path.as_ref();
    let segments = split_mbox_messages(source);

    if !source.trim().is_empty() && segments.is_empty() {
        return Err(IngestError::UnsupportedInput(
            "mailbox boundaries could not be segmented",
        ));
    }

    let source_fingerprint = prefixed_digest("src", &format!("{}:{source}", source_path.display()));
    let imported_at = Utc::now().to_rfc3339();
    let import_batch_id = prefixed_digest(
        "bat",
        &format!("{}:{source_fingerprint}", source_path.display()),
    );

    let mut participants_by_email = BTreeMap::<String, Participant>::new();
    let mut accepted_messages = Vec::new();
    let mut rejected_messages = Vec::new();
    let mut threads = Vec::<ThreadState>::new();
    let mut thread_lookup_by_message_id = HashMap::<String, String>::new();

    for (index, segment) in segments.iter().enumerate() {
        match parse_message(segment) {
            Ok(parsed) => match build_normalized_message(
                index,
                &parsed,
                &imported_at,
                &mut participants_by_email,
                &mut threads,
                &mut thread_lookup_by_message_id,
            ) {
                Ok(message) => accepted_messages.push(message),
                Err(reason) => rejected_messages.push(RejectedMessage {
                    source_index: index,
                    reason,
                }),
            },
            Err(reason) => rejected_messages.push(RejectedMessage {
                source_index: index,
                reason,
            }),
        }
    }

    let status = match (accepted_messages.is_empty(), rejected_messages.is_empty()) {
        (true, _) => ImportBatchStatus::Failed,
        (false, true) => ImportBatchStatus::Completed,
        (false, false) => ImportBatchStatus::Partial,
    };

    let threads = threads
        .into_iter()
        .map(|thread| Thread {
            thread_id: thread.thread_id,
            canonical_subject: thread.canonical_subject,
            root_message_key: thread.root_message_key,
            latest_message_at: thread.latest_message_at,
            message_count: thread.message_count,
        })
        .collect();

    Ok(ImportBatchOutput {
        import_batch_id,
        source_path: source_path.display().to_string(),
        source_fingerprint,
        imported_at,
        parser_version: PARSER_VERSION.to_string(),
        status,
        message_count_seen: segments.len(),
        accepted_messages,
        rejected_messages,
        participants: participants_by_email.into_values().collect(),
        threads,
    })
}

fn build_normalized_message(
    index: usize,
    parsed: &ParsedMessage,
    imported_at: &str,
    participants_by_email: &mut BTreeMap<String, Participant>,
    threads: &mut Vec<ThreadState>,
    thread_lookup_by_message_id: &mut HashMap<String, String>,
) -> Result<NormalizedMessage, String> {
    let sender = parsed
        .sender()
        .ok_or_else(|| "message missing sender identity".to_string())?;
    let sender = canonical_participant(sender, participants_by_email);

    let body_text = normalize_body(&parsed.body);
    let subject = parsed.header("Subject").map(normalize_whitespace);
    let canonical_subject = subject
        .as_deref()
        .map(canonicalize_subject)
        .filter(|value| !value.is_empty());
    let sent_at = parsed
        .header("Date")
        .and_then(|date| parse_timestamp(&date))
        .map(|date| date.to_rfc3339());
    let raw_message_id = parsed.header("Message-ID").map(normalize_message_id);

    if raw_message_id.is_none() && body_text.is_none() && subject.is_none() {
        return Err("message missing stable key and content signal".to_string());
    }

    let message_key = raw_message_id.clone().map_or_else(
        || {
            let fallback = format!(
                "{}:{}:{}:{}",
                sender.email,
                canonical_subject.clone().unwrap_or_default(),
                sent_at.clone().unwrap_or_default(),
                digest_hex(body_text.as_deref().unwrap_or_default())
            );
            prefixed_digest("msg", &fallback)
        },
        |message_id| prefixed_digest("msg", &format!("mid:{message_id}")),
    );

    let recipients_to = participants_for_header(parsed.header("To"), participants_by_email);
    let recipients_cc = participants_for_header(parsed.header("Cc"), participants_by_email);
    let recipients_bcc = participants_for_header(parsed.header("Bcc"), participants_by_email);
    let recipients_reply_to =
        participants_for_header(parsed.header("Reply-To"), participants_by_email);

    let body_text_digest = body_text.as_deref().map(digest_hex);
    let thread_id = assign_thread(
        ThreadAssignmentInput {
            index,
            parsed,
            message_key: &message_key,
            canonical_subject: canonical_subject.clone(),
            sent_at: sent_at.clone(),
            imported_at,
            sender: &sender,
            to: &recipients_to,
            cc: &recipients_cc,
            bcc: &recipients_bcc,
            reply_to: &recipients_reply_to,
        },
        threads,
        thread_lookup_by_message_id,
    );

    if let Some(message_id) = raw_message_id.as_ref() {
        thread_lookup_by_message_id.insert(message_id.clone(), thread_id.clone());
    }

    Ok(NormalizedMessage {
        message_key,
        raw_message_id,
        thread_id,
        subject,
        canonical_subject,
        sender_participant_id: sender.participant_id.clone(),
        sender,
        to: recipients_to,
        cc: recipients_cc,
        bcc: recipients_bcc,
        reply_to: recipients_reply_to,
        sent_at,
        body_preview: body_text.as_deref().map(preview_text),
        body_text,
        body_text_digest,
        has_html_body: false,
    })
}

fn split_mbox_messages(source: &str) -> Vec<String> {
    let normalized = source.replace("\r\n", "\n");
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for line in normalized.lines() {
        if line.starts_with("From ") && !current.is_empty() {
            segments.push(current.join("\n"));
            current.clear();
            continue;
        }

        if line.starts_with("From ") {
            continue;
        }

        current.push(line.to_string());
    }

    if !current.is_empty() {
        segments.push(current.join("\n"));
    }

    segments
}

fn parse_message(segment: &str) -> Result<ParsedMessage, String> {
    let normalized = segment.replace("\r\n", "\n");
    let (raw_headers, body) = normalized
        .split_once("\n\n")
        .ok_or_else(|| "message missing header/body separator".to_string())?;

    let mut headers = BTreeMap::<String, Vec<String>>::new();
    let mut current_name = None::<String>;

    for line in raw_headers.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            let Some(name) = current_name.as_ref() else {
                continue;
            };

            if let Some(values) = headers.get_mut(name) {
                if let Some(last) = values.last_mut() {
                    last.push(' ');
                    last.push_str(line.trim());
                }
            }
            continue;
        }

        let Some((name, value)) = line.split_once(':') else {
            continue;
        };

        let normalized_name = name.trim().to_ascii_lowercase();
        let normalized_value = value.trim().to_string();
        headers
            .entry(normalized_name.clone())
            .or_default()
            .push(normalized_value);
        current_name = Some(normalized_name);
    }

    Ok(ParsedMessage {
        headers,
        body: body.trim().to_string(),
    })
}

fn normalize_body(body: &str) -> Option<String> {
    let body = normalize_whitespace(body);
    (!body.is_empty()).then_some(body)
}

fn normalize_whitespace(value: impl AsRef<str>) -> String {
    value
        .as_ref()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_message_id(value: String) -> String {
    value
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
        .to_string()
}

fn parse_timestamp(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc2822(value.trim()).ok()
}

fn canonicalize_subject(value: &str) -> String {
    let mut subject = normalize_whitespace(value).to_ascii_lowercase();

    loop {
        let trimmed = subject.trim_start();
        let mut next = None;

        for prefix in ["re:", "fw:", "fwd:"] {
            if let Some(rest) = trimmed.strip_prefix(prefix) {
                next = Some(rest.trim_start().to_string());
                break;
            }
        }

        match next {
            Some(updated) => subject = updated,
            None => return trimmed.to_string(),
        }
    }
}

fn preview_text(value: &str) -> String {
    const LIMIT: usize = 80;
    let preview: String = value.chars().take(LIMIT).collect();
    preview
}

fn participants_for_header(
    value: Option<String>,
    participants_by_email: &mut BTreeMap<String, Participant>,
) -> Vec<Participant> {
    let Some(value) = value else {
        return Vec::new();
    };

    split_addresses(&value)
        .into_iter()
        .map(|address| canonical_participant(address, participants_by_email))
        .collect()
}

fn canonical_participant(
    address: MailboxAddress,
    participants_by_email: &mut BTreeMap<String, Participant>,
) -> Participant {
    if let Some(existing) = participants_by_email.get(&address.email) {
        return existing.clone();
    }

    let participant = Participant {
        participant_id: prefixed_digest("par", &address.email),
        email: address.email,
        display_name: address.display_name,
    };

    participants_by_email.insert(participant.email.clone(), participant.clone());
    participant
}

fn assign_thread(
    input: ThreadAssignmentInput<'_>,
    threads: &mut Vec<ThreadState>,
    thread_lookup_by_message_id: &HashMap<String, String>,
) -> String {
    let sent_at_parsed = input.sent_at.as_deref().and_then(parse_rfc3339_fixed);
    let imported_at_parsed = parse_rfc3339_fixed(input.imported_at);

    let thread_id = input
        .parsed
        .header("In-Reply-To")
        .map(normalize_message_id)
        .and_then(|message_id| thread_lookup_by_message_id.get(&message_id).cloned())
        .or_else(|| {
            input
                .parsed
                .references()
                .iter()
                .rev()
                .find_map(|reference| thread_lookup_by_message_id.get(reference).cloned())
        })
        .or_else(|| {
            let participant_ids =
                thread_participant_ids(input.sender, input.to, input.cc, input.bcc, input.reply_to);

            threads.iter().find_map(|thread| {
                if thread.canonical_subject != input.canonical_subject {
                    return None;
                }

                if thread.participant_ids.is_disjoint(&participant_ids) {
                    return None;
                }

                let candidate_time = sent_at_parsed.or(imported_at_parsed);
                let latest_time = thread
                    .latest_message_at
                    .as_deref()
                    .and_then(parse_rfc3339_fixed);

                match (candidate_time, latest_time) {
                    (Some(candidate_time), Some(latest_time))
                        if (candidate_time - latest_time).num_days().abs() <= 14 =>
                    {
                        Some(thread.thread_id.clone())
                    }
                    (None, _) | (_, None) => Some(thread.thread_id.clone()),
                    _ => None,
                }
            })
        })
        .unwrap_or_else(|| {
            prefixed_digest("thr", &format!("{}:{}", input.index, input.message_key))
        });

    if let Some(thread) = threads
        .iter_mut()
        .find(|thread| thread.thread_id == thread_id)
    {
        thread.message_count += 1;
        if thread.root_message_key.is_empty() {
            thread.root_message_key = input.message_key.to_string();
        }

        if newer_timestamp(
            input.sent_at.as_deref(),
            thread.latest_message_at.as_deref(),
        ) {
            thread.latest_message_at.clone_from(&input.sent_at);
        }

        thread.participant_ids.extend(thread_participant_ids(
            input.sender,
            input.to,
            input.cc,
            input.bcc,
            input.reply_to,
        ));
    } else {
        threads.push(ThreadState {
            thread_id: thread_id.clone(),
            canonical_subject: input.canonical_subject,
            root_message_key: input.message_key.to_string(),
            latest_message_at: input.sent_at,
            message_count: 1,
            participant_ids: thread_participant_ids(
                input.sender,
                input.to,
                input.cc,
                input.bcc,
                input.reply_to,
            ),
        });
    }

    thread_id
}

fn newer_timestamp(candidate: Option<&str>, existing: Option<&str>) -> bool {
    match (
        candidate.and_then(parse_rfc3339_fixed),
        existing.and_then(parse_rfc3339_fixed),
    ) {
        (Some(candidate), Some(existing)) => candidate > existing,
        (Some(_), None) => true,
        _ => false,
    }
}

fn thread_participant_ids(
    sender: &Participant,
    to: &[Participant],
    cc: &[Participant],
    bcc: &[Participant],
    reply_to: &[Participant],
) -> BTreeSet<String> {
    let mut ids = BTreeSet::from([sender.participant_id.clone()]);

    ids.extend(
        to.iter()
            .map(|participant| participant.participant_id.clone()),
    );
    ids.extend(
        cc.iter()
            .map(|participant| participant.participant_id.clone()),
    );
    ids.extend(
        bcc.iter()
            .map(|participant| participant.participant_id.clone()),
    );
    ids.extend(
        reply_to
            .iter()
            .map(|participant| participant.participant_id.clone()),
    );

    ids
}

fn parse_rfc3339_fixed(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

fn split_addresses(value: &str) -> Vec<MailboxAddress> {
    value.split(',').filter_map(parse_address).collect()
}

fn parse_address(value: &str) -> Option<MailboxAddress> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some((display_name, email)) = value.rsplit_once('<') {
        let email = email.trim_end_matches('>').trim().to_ascii_lowercase();
        if email.is_empty() {
            return None;
        }

        let display_name = display_name.trim().trim_matches('"').trim().to_string();
        return Some(MailboxAddress {
            email,
            display_name: (!display_name.is_empty()).then_some(display_name),
        });
    }

    Some(MailboxAddress {
        email: value.trim_matches('"').to_ascii_lowercase(),
        display_name: None,
    })
}

fn prefixed_digest(prefix: &str, value: &str) -> String {
    format!("{prefix}_{}", &digest_hex(value)[..16])
}

fn digest_hex(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[derive(Debug, Clone)]
struct ParsedMessage {
    headers: BTreeMap<String, Vec<String>>,
    body: String,
}

impl ParsedMessage {
    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .and_then(|values| values.first())
            .cloned()
    }

    fn sender(&self) -> Option<MailboxAddress> {
        self.header("From")
            .or_else(|| self.header("Sender"))
            .and_then(|value| split_addresses(&value).into_iter().next())
    }

    fn references(&self) -> Vec<String> {
        self.header("References")
            .map(|value| {
                value
                    .split_whitespace()
                    .map(|reference| normalize_message_id(reference.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
struct MailboxAddress {
    email: String,
    display_name: Option<String>,
}

#[derive(Debug, Clone)]
struct ThreadState {
    thread_id: String,
    canonical_subject: Option<String>,
    root_message_key: String,
    latest_message_at: Option<String>,
    message_count: usize,
    participant_ids: BTreeSet<String>,
}

struct ThreadAssignmentInput<'a> {
    index: usize,
    parsed: &'a ParsedMessage,
    message_key: &'a str,
    canonical_subject: Option<String>,
    sent_at: Option<String>,
    imported_at: &'a str,
    sender: &'a Participant,
    to: &'a [Participant],
    cc: &'a [Participant],
    bcc: &'a [Participant],
    reply_to: &'a [Participant],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_subject_strips_reply_prefixes() {
        assert_eq!(
            canonicalize_subject("Re: Fwd: Can you review the investor update?"),
            "can you review the investor update?"
        );
    }

    #[test]
    fn fixture_source_imports_successfully() {
        let output = import_mbox_source(
            "fixtures/mailbox/minimal-thread.mbox",
            "From founder@example.com Mon Apr 13 08:15:00 2026\nSubject: Can you review the investor update?\nFrom: Founder <founder@example.com>\nTo: Operator <operator@example.com>\nDate: Mon, 13 Apr 2026 08:15:00 +0000\nMessage-ID: <review-thread@example.com>\n\nCan you review this before tomorrow morning?\n",
        )
        .unwrap();

        assert_eq!(output.status, ImportBatchStatus::Completed);
        assert_eq!(output.accepted_messages.len(), 1);
        assert_eq!(
            output.accepted_messages[0].canonical_subject.as_deref(),
            Some("can you review the investor update?")
        );
    }
}
