#[test]
fn ai_scope_mentions_optional_adapter() {
    assert!(briefly_ai::bootstrap_scope().contains("optional"));
}
