use std::path::PathBuf;

use briefly_contracts::{
    ImportBatchOutput, ImportBatchStatus, NormalizedMessage, Participant, RejectedMessage, Thread,
};

fn fixture_output() -> ImportBatchOutput {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/mailbox/minimal-thread.mbox");

    briefly_ingest::import_mbox_fixture(fixture_path).expect("fixture import should succeed")
}

#[test]
fn persisting_fixture_import_writes_canonical_rows_and_provenance() {
    let mut store = briefly_store::Store::open_in_memory().expect("store should initialize");
    let output = fixture_output();

    let report = store
        .persist_import_batch(&output)
        .expect("import persistence should succeed");

    assert_eq!(report.linked_messages, 1);
    assert_eq!(report.parse_error_count, 0);

    let connection = store.connection();

    let import_batch_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM import_batches", [], |row| row.get(0))
        .expect("import batch count should query");
    let message_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        .expect("message count should query");
    let participant_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM participants", [], |row| row.get(0))
        .expect("participant count should query");
    let thread_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM threads", [], |row| row.get(0))
        .expect("thread count should query");
    let message_source_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM message_sources", [], |row| row.get(0))
        .expect("source count should query");
    let message_participant_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM message_participants", [], |row| row.get(0))
        .expect("participant links should query");

    assert_eq!(import_batch_count, 1);
    assert_eq!(message_count, 1);
    assert_eq!(participant_count, 2);
    assert_eq!(thread_count, 1);
    assert_eq!(message_source_count, 1);
    assert_eq!(message_participant_count, 1);

    let first_seen_batch_id: String = connection
        .query_row(
            "SELECT import_first_seen_batch_id FROM messages LIMIT 1",
            [],
            |row| row.get(0),
        )
        .expect("message should keep first seen batch id");
    assert_eq!(first_seen_batch_id, report.import_batch_id);
}

#[test]
fn repeated_imports_add_batches_without_duplicating_canonical_rows() {
    let mut store = briefly_store::Store::open_in_memory().expect("store should initialize");
    let first = fixture_output();
    std::thread::sleep(std::time::Duration::from_millis(5));
    let second = fixture_output();

    store
        .persist_import_batch(&first)
        .expect("first import should persist");
    store
        .persist_import_batch(&second)
        .expect("second import should persist");

    let connection = store.connection();

    let import_batch_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM import_batches", [], |row| row.get(0))
        .expect("import batch count should query");
    let message_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
        .expect("message count should query");
    let participant_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM participants", [], |row| row.get(0))
        .expect("participant count should query");
    let message_source_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM message_sources", [], |row| row.get(0))
        .expect("source count should query");

    assert_eq!(import_batch_count, 2);
    assert_eq!(message_count, 1);
    assert_eq!(participant_count, 2);
    assert_eq!(message_source_count, 2);
}

#[test]
fn partial_imports_keep_diagnostics_queryable() {
    let mut store = briefly_store::Store::open_in_memory().expect("store should initialize");
    let accepted_sender = Participant {
        participant_id: "par_sender".to_string(),
        email: "sender@example.com".to_string(),
        display_name: Some("Sender".to_string()),
    };
    let accepted_recipient = Participant {
        participant_id: "par_recipient".to_string(),
        email: "recipient@example.com".to_string(),
        display_name: Some("Recipient".to_string()),
    };
    let output = ImportBatchOutput {
        import_batch_id: "bat_partial".to_string(),
        source_path: "/tmp/partial.mbox".to_string(),
        source_fingerprint: "src_partial".to_string(),
        imported_at: "2026-04-15T12:34:56Z".to_string(),
        parser_version: "briefly-ingest/0.1.0".to_string(),
        status: ImportBatchStatus::Partial,
        message_count_seen: 2,
        accepted_messages: vec![NormalizedMessage {
            message_key: "msg_partial".to_string(),
            raw_message_id: Some("partial@example.com".to_string()),
            thread_id: "thr_partial".to_string(),
            subject: Some("Hello".to_string()),
            canonical_subject: Some("hello".to_string()),
            sender_participant_id: accepted_sender.participant_id.clone(),
            sender: accepted_sender.clone(),
            to: vec![accepted_recipient.clone()],
            cc: vec![],
            bcc: vec![],
            reply_to: vec![],
            sent_at: Some("2026-04-15T12:00:00Z".to_string()),
            body_text: Some("Hi there".to_string()),
            body_preview: Some("Hi there".to_string()),
            body_text_digest: Some("digest".to_string()),
            has_html_body: false,
        }],
        rejected_messages: vec![RejectedMessage {
            source_index: 1,
            reason: "missing sender identity".to_string(),
        }],
        participants: vec![accepted_sender, accepted_recipient],
        threads: vec![Thread {
            thread_id: "thr_partial".to_string(),
            canonical_subject: Some("hello".to_string()),
            root_message_key: "msg_partial".to_string(),
            latest_message_at: Some("2026-04-15T12:00:00Z".to_string()),
            message_count: 1,
        }],
    };

    let report = store
        .persist_import_batch(&output)
        .expect("partial import should persist");

    assert_eq!(report.linked_messages, 1);
    assert_eq!(report.parse_error_count, 1);

    let connection = store.connection();
    let batch_row: (String, i64, i64, Option<String>) = connection
        .query_row(
            "SELECT status, message_count_linked, parse_error_count, notes FROM import_batches LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("batch diagnostics should query");
    let source_rows: Vec<(Option<String>, String, Option<String>, i64)> = {
        let mut statement = connection
            .prepare(
                "SELECT message_id, parse_status, parse_error, source_position
                 FROM message_sources
                 ORDER BY source_position",
            )
            .expect("source query should prepare");
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .expect("source query should run")
            .collect::<Result<Vec<_>, _>>()
            .expect("source rows should collect")
    };

    assert_eq!(batch_row.0, "partial");
    assert_eq!(batch_row.1, 1);
    assert_eq!(batch_row.2, 1);
    assert!(batch_row.3.expect("notes should exist").contains("missing sender identity"));
    assert_eq!(source_rows.len(), 2);
    assert_eq!(source_rows[0].1, "parsed");
    assert!(source_rows[0].0.is_some());
    assert_eq!(source_rows[0].3, 0);
    assert_eq!(source_rows[1].1, "failed");
    assert_eq!(source_rows[1].2.as_deref(), Some("missing sender identity"));
    assert_eq!(source_rows[1].3, 1);
}
