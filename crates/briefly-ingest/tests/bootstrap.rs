#[test]
fn ingest_scope_mentions_normalization() {
    assert!(briefly_ingest::bootstrap_scope().contains("normalization"));
}
