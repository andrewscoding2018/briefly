use rusqlite::Connection;
use std::path::Path;

const INITIAL_SCHEMA: &str = include_str!("../migrations/0001_initial_schema.sql");
const SCHEMA_VERSION: i32 = 1;

pub fn bootstrap_scope() -> &'static str {
    "sqlite access, migrations, and persistence repositories"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Migration {
    pub version: i32,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: [Migration; 1] = [Migration {
    version: SCHEMA_VERSION,
    name: "0001_initial_schema",
    sql: INITIAL_SCHEMA,
}];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepositoryBoundary {
    CanonicalMail,
    DerivedReadModels,
}

impl RepositoryBoundary {
    pub fn description(self) -> &'static str {
        match self {
            Self::CanonicalMail => {
                "Owns import provenance plus canonical participants, threads, messages, and message links."
            }
            Self::DerivedReadModels => {
                "Owns scoring runs, relationship snapshots, thread scores, and briefing snapshots."
            }
        }
    }
}

pub const REPOSITORY_BOUNDARIES: [RepositoryBoundary; 2] = [
    RepositoryBoundary::CanonicalMail,
    RepositoryBoundary::DerivedReadModels,
];

pub struct Store {
    connection: Connection,
}

impl Store {
    pub fn open_path(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        Self::initialize(connection)
    }

    pub fn open_in_memory() -> rusqlite::Result<Self> {
        let connection = Connection::open_in_memory()?;
        Self::initialize(connection)
    }

    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    fn initialize(mut connection: Connection) -> rusqlite::Result<Self> {
        connection.pragma_update(None, "foreign_keys", 1)?;
        apply_migrations(&mut connection)?;
        Ok(Self { connection })
    }
}

pub fn apply_migrations(connection: &mut Connection) -> rusqlite::Result<()> {
    let tx = connection.transaction()?;
    let current_version: i32 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;

    for migration in MIGRATIONS
        .iter()
        .filter(|migration| migration.version > current_version)
    {
        tx.execute_batch(migration.sql)?;
        tx.pragma_update(None, "user_version", migration.version)?;
    }

    tx.commit()?;
    Ok(())
}

pub trait CanonicalRepository {
    fn import_batches_table(&self) -> &'static str {
        "import_batches"
    }

    fn message_sources_table(&self) -> &'static str {
        "message_sources"
    }

    fn participants_table(&self) -> &'static str {
        "participants"
    }

    fn threads_table(&self) -> &'static str {
        "threads"
    }

    fn messages_table(&self) -> &'static str {
        "messages"
    }

    fn message_participants_table(&self) -> &'static str {
        "message_participants"
    }
}

pub trait DerivedRepository {
    fn scoring_runs_table(&self) -> &'static str {
        "scoring_runs"
    }

    fn participant_relationship_scores_table(&self) -> &'static str {
        "participant_relationship_scores"
    }

    fn signal_scores_table(&self) -> &'static str {
        "signal_scores"
    }

    fn briefing_entries_table(&self) -> &'static str {
        "briefing_entries"
    }
}

pub struct SqliteRepositories;

impl CanonicalRepository for SqliteRepositories {}
impl DerivedRepository for SqliteRepositories {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_boundaries_are_split_between_canonical_and_derived_data() {
        assert_eq!(REPOSITORY_BOUNDARIES.len(), 2);
        assert!(REPOSITORY_BOUNDARIES.contains(&RepositoryBoundary::CanonicalMail));
        assert!(REPOSITORY_BOUNDARIES
            .iter()
            .any(|boundary| { boundary.description().contains("scoring runs") }));
    }

    #[test]
    fn sqlite_store_bootstrap_reaches_schema_version() {
        let store = Store::open_in_memory().expect("in-memory store should initialize");
        let version: i32 = store
            .connection()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("schema version should be queryable");

        assert_eq!(version, SCHEMA_VERSION);
    }
}
