#[test]
fn score_scope_mentions_explanations() {
    assert!(briefly_score::bootstrap_scope().contains("explanation"));
}
