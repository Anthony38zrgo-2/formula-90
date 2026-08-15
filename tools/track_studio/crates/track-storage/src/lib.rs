use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::path::Path;
use track_domain::TrackDocument;

const STORAGE_SCHEMA_VERSION: i64 = 1;
const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");

#[derive(Debug)]
pub enum StorageError {
    Sqlite(rusqlite::Error),
    Json(serde_json::Error),
    InvalidDocument(Vec<track_domain::Diagnostic>),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "sqlite: {error}"),
            Self::Json(error) => write!(formatter, "json: {error}"),
            Self::InvalidDocument(diagnostics) => {
                write!(
                    formatter,
                    "document validation failed: {} diagnostic(s)",
                    diagnostics.len()
                )
            }
        }
    }
}

impl std::error::Error for StorageError {}

impl From<rusqlite::Error> for StorageError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

pub struct TrackStorage {
    connection: Connection,
}

impl TrackStorage {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let connection = Connection::open(path)?;
        let storage = Self { connection };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn open_in_memory() -> Result<Self, StorageError> {
        let connection = Connection::open_in_memory()?;
        let storage = Self { connection };
        storage.migrate()?;
        Ok(storage)
    }

    pub fn schema_version(&self) -> Result<i64, StorageError> {
        self.connection
            .query_row(
                "SELECT value FROM storage_metadata WHERE key = 'schema_version'",
                [],
                |row| row.get::<_, String>(0),
            )?
            .parse::<i64>()
            .map_err(|error| {
                StorageError::Sqlite(rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                ))
            })
    }

    pub fn save(&self, document: &TrackDocument) -> Result<(), StorageError> {
        let diagnostics = document.validate();
        if !diagnostics.is_empty() {
            return Err(StorageError::InvalidDocument(diagnostics));
        }
        let json =
            String::from_utf8(document.canonical_json()?).expect("canonical JSON is always UTF-8");
        let transaction = self.connection.unchecked_transaction()?;
        save_in_transaction(&transaction, document, &json)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load(&self, track_id: &str) -> Result<Option<TrackDocument>, StorageError> {
        let json = self
            .connection
            .query_row(
                "SELECT document_json FROM project WHERE track_id = ?1",
                params![track_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        json.map(|value| serde_json::from_str(&value).map_err(StorageError::from))
            .transpose()
    }

    pub fn load_any_track_id(&self) -> Result<String, StorageError> {
        self.connection
            .query_row("SELECT track_id FROM project LIMIT 1", [], |row| {
                row.get::<_, String>(0)
            })
            .map_err(Into::into)
    }

    fn migrate(&self) -> Result<(), StorageError> {
        self.connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 1000;")?;
        self.connection.execute_batch(INITIAL_MIGRATION)?;
        debug_assert_eq!(self.schema_version()?, STORAGE_SCHEMA_VERSION);
        Ok(())
    }
}

fn save_in_transaction(
    transaction: &Transaction<'_>,
    document: &TrackDocument,
    json: &str,
) -> Result<(), StorageError> {
    transaction.execute(
        "INSERT INTO project(track_id, schema_version, project_version, compiler_version, document_json)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(track_id) DO UPDATE SET
           schema_version = excluded.schema_version,
           project_version = excluded.project_version,
           compiler_version = excluded.compiler_version,
           document_json = excluded.document_json,
           updated_at = CURRENT_TIMESTAMP",
        params![
            document.track_id,
            document.schema_version,
            document.project_version,
            document.compiler_version,
            json
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use track_domain::{ControlPoint, TrackDocument};

    fn document() -> TrackDocument {
        let mut document = TrackDocument::new("test");
        document.centerline = vec![
            ControlPoint {
                id: "a".into(),
                x: 0.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "b".into(),
                x: 10.0,
                z: 0.0,
                handle_in: None,
                handle_out: None,
            },
            ControlPoint {
                id: "c".into(),
                x: 10.0,
                z: 10.0,
                handle_in: None,
                handle_out: None,
            },
        ];
        document
    }

    #[test]
    fn migration_and_round_trip_preserve_document() {
        let storage = TrackStorage::open_in_memory().unwrap();
        assert_eq!(storage.schema_version().unwrap(), 1);
        let original = document();
        storage.save(&original).unwrap();
        let loaded = storage.load("test").unwrap().unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn save_replaces_existing_document_transactionally() {
        let storage = TrackStorage::open_in_memory().unwrap();
        let mut original = document();
        storage.save(&original).unwrap();
        original.project_version = "0.2.0".into();
        storage.save(&original).unwrap();
        assert_eq!(
            storage.load("test").unwrap().unwrap().project_version,
            "0.2.0"
        );
    }

    #[test]
    fn invalid_document_is_rejected_before_persistence() {
        let storage = TrackStorage::open_in_memory().unwrap();
        let invalid = TrackDocument::new("invalid");
        assert!(matches!(
            storage.save(&invalid),
            Err(StorageError::InvalidDocument(_))
        ));
        assert!(storage.load("invalid").unwrap().is_none());
    }

    #[test]
    fn file_backed_project_reopens_with_saved_document() {
        let path = std::env::temp_dir().join(format!(
            "formula90_track_storage_{}.f90track",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        {
            let storage = TrackStorage::open(&path).unwrap();
            storage.save(&document()).unwrap();
        }
        let reopened = TrackStorage::open(&path).unwrap();
        assert_eq!(reopened.load("test").unwrap().unwrap(), document());
        drop(reopened);
        std::fs::remove_file(path).unwrap();
    }
}
