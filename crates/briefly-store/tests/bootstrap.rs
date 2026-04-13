#[test]
fn store_scope_mentions_migrations() {
    assert!(briefly_store::bootstrap_scope().contains("migrations"));
}
