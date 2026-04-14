use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn store_scope_mentions_migrations() {
    assert!(briefly_store::bootstrap_scope().contains("migrations"));
}

#[test]
fn file_backed_store_initializes_documented_phase_one_tables() {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    let db_path = std::env::temp_dir().join(format!("briefly-store-{unique_suffix}.sqlite3"));

    let store = briefly_store::Store::open_path(&db_path).expect("store should initialize");

    let table_names = {
        let mut stmt = store
            .connection()
            .prepare(
                "SELECT name
                 FROM sqlite_master
                 WHERE type = 'table' AND name NOT LIKE 'sqlite_%'
                 ORDER BY name",
            )
            .expect("schema query should prepare");

        stmt.query_map([], |row| row.get::<_, String>(0))
            .expect("schema query should execute")
            .collect::<Result<Vec<_>, _>>()
            .expect("table names should collect")
    };

    assert_eq!(
        table_names,
        vec![
            "briefing_entries",
            "import_batches",
            "message_participants",
            "message_sources",
            "messages",
            "participant_relationship_scores",
            "participants",
            "scoring_runs",
            "signal_scores",
            "threads",
        ]
    );

    drop(store);
    std::fs::remove_file(db_path).expect("temporary sqlite file should be removed");
}
