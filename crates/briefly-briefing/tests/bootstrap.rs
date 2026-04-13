#[test]
fn briefing_scope_mentions_read_models() {
    assert!(briefly_briefing::bootstrap_scope().contains("read models"));
}
