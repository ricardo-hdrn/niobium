use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use tracing::debug;

use crate::error::NiobiumError;

/// Versioned form schema storage backed by SQLite.
pub struct SchemaStore {
    conn: Connection,
}

/// A saved form schema.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SavedForm {
    pub id: i64,
    pub name: String,
    pub version: i64,
    pub schema_json: serde_json::Value,
    pub description: String,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub use_count: i64,
}

/// A recorded form submission.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FormSubmission {
    pub id: i64,
    pub form_name: Option<String>,
    pub form_version: Option<i64>,
    pub response_json: serde_json::Value,
    pub submitted_at: String,
}

impl SchemaStore {
    /// Open or create the SQLite database at the given path.
    pub fn open(db_path: &Path) -> Result<Self, NiobiumError> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| NiobiumError::StorageError(format!("cannot create db dir: {e}")))?;
        }

        let conn = Connection::open(db_path)?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    /// Open an in-memory database (for testing).
    #[cfg(test)]
    pub fn open_memory() -> Result<Self, NiobiumError> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), NiobiumError> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS forms (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                schema_json TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_used_at TEXT,
                use_count INTEGER NOT NULL DEFAULT 0,
                UNIQUE(name, version)
            );

            CREATE TABLE IF NOT EXISTS form_submissions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                form_name TEXT,
                form_version INTEGER,
                response_json TEXT NOT NULL,
                submitted_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;
        Ok(())
    }

    /// Save a form schema. Auto-increments version if name already exists.
    pub fn save_form(
        &self,
        name: &str,
        schema: &serde_json::Value,
        description: &str,
    ) -> Result<SavedForm, NiobiumError> {
        let next_version: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) + 1 FROM forms WHERE name = ?1",
            params![name],
            |row| row.get(0),
        )?;

        let schema_str = serde_json::to_string(schema)
            .map_err(|e| NiobiumError::StorageError(format!("cannot serialize schema: {e}")))?;

        self.conn.execute(
            "INSERT INTO forms (name, version, schema_json, description) VALUES (?1, ?2, ?3, ?4)",
            params![name, next_version, schema_str, description],
        )?;

        let id = self.conn.last_insert_rowid();
        debug!(name, version = next_version, id, "saved form schema");

        Ok(SavedForm {
            id,
            name: name.to_string(),
            version: next_version,
            schema_json: schema.clone(),
            description: description.to_string(),
            created_at: String::new(), // Will be set by SQLite
            last_used_at: None,
            use_count: 0,
        })
    }

    /// Get the latest version of a saved form by name.
    pub fn get_form(&self, name: &str) -> Result<Option<SavedForm>, NiobiumError> {
        let result = self
            .conn
            .query_row(
                "SELECT id, name, version, schema_json, description, created_at, last_used_at, use_count
                 FROM forms WHERE name = ?1 ORDER BY version DESC LIMIT 1",
                params![name],
                |row| {
                    let schema_str: String = row.get(3)?;
                    Ok(SavedForm {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        version: row.get(2)?,
                        schema_json: serde_json::from_str(&schema_str).unwrap_or_default(),
                        description: row.get(4)?,
                        created_at: row.get(5)?,
                        last_used_at: row.get(6)?,
                        use_count: row.get(7)?,
                    })
                },
            )
            .optional()?;

        Ok(result)
    }

    /// List all saved forms (latest version of each).
    pub fn list_forms(&self) -> Result<Vec<SavedForm>, NiobiumError> {
        let mut stmt = self.conn.prepare(
            "SELECT f.id, f.name, f.version, f.schema_json, f.description,
                    f.created_at, f.last_used_at, f.use_count
             FROM forms f
             INNER JOIN (SELECT name, MAX(version) as max_ver FROM forms GROUP BY name) latest
             ON f.name = latest.name AND f.version = latest.max_ver
             ORDER BY f.last_used_at DESC NULLS LAST, f.created_at DESC",
        )?;

        let forms = stmt
            .query_map([], |row| {
                let schema_str: String = row.get(3)?;
                Ok(SavedForm {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    version: row.get(2)?,
                    schema_json: serde_json::from_str(&schema_str).unwrap_or_default(),
                    description: row.get(4)?,
                    created_at: row.get(5)?,
                    last_used_at: row.get(6)?,
                    use_count: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(forms)
    }

    /// Record a form usage (increments use_count, updates last_used_at).
    pub fn record_usage(&self, name: &str, version: i64) -> Result<(), NiobiumError> {
        self.conn.execute(
            "UPDATE forms SET use_count = use_count + 1, last_used_at = datetime('now')
             WHERE name = ?1 AND version = ?2",
            params![name, version],
        )?;
        Ok(())
    }

    /// Record a form submission.
    pub fn record_submission(
        &self,
        form_name: Option<&str>,
        form_version: Option<i64>,
        response: &serde_json::Value,
    ) -> Result<(), NiobiumError> {
        let response_str = serde_json::to_string(response)
            .map_err(|e| NiobiumError::StorageError(format!("cannot serialize response: {e}")))?;

        self.conn.execute(
            "INSERT INTO form_submissions (form_name, form_version, response_json) VALUES (?1, ?2, ?3)",
            params![form_name, form_version, response_str],
        )?;
        Ok(())
    }

    /// Get the most recent submission for a given form name (for pre-fill).
    pub fn last_submission(
        &self,
        form_name: &str,
    ) -> Result<Option<serde_json::Value>, NiobiumError> {
        let result = self
            .conn
            .query_row(
                "SELECT response_json FROM form_submissions
                 WHERE form_name = ?1 ORDER BY submitted_at DESC LIMIT 1",
                params![form_name],
                |row| {
                    let json_str: String = row.get(0)?;
                    Ok(serde_json::from_str(&json_str).unwrap_or_default())
                },
            )
            .optional()?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_save_and_get_form() {
        let store = SchemaStore::open_memory().unwrap();

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        });

        let saved = store
            .save_form("test-form", &schema, "A test form")
            .unwrap();
        assert_eq!(saved.name, "test-form");
        assert_eq!(saved.version, 1);

        let fetched = store.get_form("test-form").unwrap().unwrap();
        assert_eq!(fetched.name, "test-form");
        assert_eq!(fetched.version, 1);
        assert_eq!(fetched.schema_json, schema);
    }

    #[test]
    fn test_auto_increments_version() {
        let store = SchemaStore::open_memory().unwrap();
        let schema = json!({"type": "object"});

        let v1 = store.save_form("my-form", &schema, "v1").unwrap();
        assert_eq!(v1.version, 1);

        let v2 = store.save_form("my-form", &schema, "v2").unwrap();
        assert_eq!(v2.version, 2);

        // get_form returns latest
        let latest = store.get_form("my-form").unwrap().unwrap();
        assert_eq!(latest.version, 2);
        assert_eq!(latest.description, "v2");
    }

    #[test]
    fn test_list_forms_returns_latest_versions() {
        let store = SchemaStore::open_memory().unwrap();
        let schema = json!({"type": "object"});

        store.save_form("alpha", &schema, "").unwrap();
        store.save_form("alpha", &schema, "").unwrap();
        store.save_form("beta", &schema, "").unwrap();

        let forms = store.list_forms().unwrap();
        assert_eq!(forms.len(), 2);

        let names: Vec<&str> = forms.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn test_get_nonexistent_form_returns_none() {
        let store = SchemaStore::open_memory().unwrap();
        assert!(store.get_form("nope").unwrap().is_none());
    }

    #[test]
    fn test_record_and_get_submission() {
        let store = SchemaStore::open_memory().unwrap();
        let response = json!({"name": "Alice", "age": 30});

        store
            .record_submission(Some("contact"), Some(1), &response)
            .unwrap();

        let last = store.last_submission("contact").unwrap().unwrap();
        assert_eq!(last, response);
    }
}
