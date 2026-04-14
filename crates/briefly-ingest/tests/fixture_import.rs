use std::path::PathBuf;

use briefly_contracts::ImportBatchStatus;

#[test]
fn minimal_thread_fixture_imports_deterministically() {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/mailbox/minimal-thread.mbox");

    let output = briefly_ingest::import_mbox_fixture(fixture_path).unwrap();

    assert_eq!(output.status, ImportBatchStatus::Completed);
    assert_eq!(output.message_count_seen, 1);
    assert!(output.rejected_messages.is_empty());
    assert_eq!(output.accepted_messages.len(), 1);
    assert_eq!(output.threads.len(), 1);
    assert_eq!(output.participants.len(), 2);

    let message = &output.accepted_messages[0];
    let thread = &output.threads[0];

    assert_eq!(
        message.raw_message_id.as_deref(),
        Some("review-thread@example.com")
    );
    assert_eq!(
        message.subject.as_deref(),
        Some("Can you review the investor update?")
    );
    assert_eq!(
        message.canonical_subject.as_deref(),
        Some("can you review the investor update?")
    );
    assert_eq!(
        message.sent_at.as_deref(),
        Some("2026-04-13T08:15:00+00:00")
    );
    assert_eq!(
        message.body_preview.as_deref(),
        Some("Can you review this before tomorrow morning?")
    );
    assert_eq!(message.sender.display_name.as_deref(), Some("Founder"));
    assert_eq!(message.sender.email, "founder@example.com");
    assert_eq!(message.to.len(), 1);
    assert_eq!(message.to[0].email, "operator@example.com");

    assert_eq!(thread.thread_id, message.thread_id);
    assert_eq!(thread.root_message_key, message.message_key);
    assert_eq!(thread.message_count, 1);
    assert_eq!(
        thread.canonical_subject.as_deref(),
        Some("can you review the investor update?")
    );

    let expected_message_key = "msg_9b02027992edcbe7";
    let expected_thread_id = "thr_b603d6abb10465ac";
    let expected_sender_id = "par_fb72704240c3527b";
    let expected_recipient_id = "par_5da87e12d60dd043";

    assert_eq!(message.message_key, expected_message_key);
    assert_eq!(message.thread_id, expected_thread_id);
    assert_eq!(message.sender_participant_id, expected_sender_id);
    assert_eq!(message.sender.participant_id, expected_sender_id);
    assert_eq!(message.to[0].participant_id, expected_recipient_id);
}
