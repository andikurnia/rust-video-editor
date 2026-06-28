use rusqlite::{params, Connection, Result as SqlResult};

use editor_core::Project;

fn current_timestamp() -> String {
    chrono_now()
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", d.as_secs())
}

#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct ProjectDatabase {
    conn: Connection,
}

impl ProjectDatabase {
    pub fn open(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        Ok(ProjectDatabase { conn })
    }

    pub fn open_in_memory() -> SqlResult<Self> {
        let conn = Connection::open_in_memory()?;
        Ok(ProjectDatabase { conn })
    }

    pub fn initialize_schema(&self) -> SqlResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                project_data TEXT NOT NULL DEFAULT '{}'
            );

            CREATE TABLE IF NOT EXISTS media_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER NOT NULL,
                path TEXT NOT NULL,
                name TEXT NOT NULL,
                duration REAL NOT NULL DEFAULT 0.0,
                width INTEGER NOT NULL DEFAULT 0,
                height INTEGER NOT NULL DEFAULT 0,
                frame_rate REAL NOT NULL DEFAULT 0.0,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS export_presets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id INTEGER,
                name TEXT NOT NULL,
                width INTEGER NOT NULL,
                height INTEGER NOT NULL,
                frame_rate REAL NOT NULL,
                bitrate TEXT NOT NULL,
                codec TEXT NOT NULL,
                container TEXT NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
            );
            ",
        )?;
        Ok(())
    }

    pub fn save_project(&self, project: &Project) -> SqlResult<i64> {
        let data = serde_json::to_string(project).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })?;

        let now = current_timestamp();

        let existing: Option<i64> = self
            .conn
            .query_row(
                "SELECT id FROM projects WHERE name = ?1 ORDER BY id DESC LIMIT 1",
                params![project.name],
                |row| row.get(0),
            )
            .ok();

        match existing {
            Some(id) => {
                self.conn.execute(
                    "UPDATE projects SET project_data = ?1, updated_at = ?2 WHERE id = ?3",
                    params![data, now, id],
                )?;
                self.sync_media_items(id, project)?;
                Ok(id)
            }
            None => {
                self.conn.execute(
                    "INSERT INTO projects (name, created_at, updated_at, project_data) VALUES (?1, ?2, ?2, ?3)",
                    params![project.name, now, data],
                )?;
                let id = self.conn.last_insert_rowid();
                self.sync_media_items(id, project)?;
                Ok(id)
            }
        }
    }

    pub fn load_project(&self, id: i64) -> SqlResult<Project> {
        let data: String = self.conn.query_row(
            "SELECT project_data FROM projects WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        serde_json::from_str(&data).map_err(|e| {
            rusqlite::Error::ToSqlConversionFailure(Box::new(e))
        })
    }

    pub fn list_projects(&self) -> SqlResult<Vec<ProjectSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, created_at, updated_at FROM projects ORDER BY updated_at DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        })?;
        rows.collect()
    }

    pub fn delete_project(&self, id: i64) -> SqlResult<()> {
        self.conn.execute("DELETE FROM projects WHERE id = ?1", params![id])?;
        Ok(())
    }

    fn sync_media_items(&self, project_id: i64, project: &Project) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM media_items WHERE project_id = ?1",
            params![project_id],
        )?;
        for item in &project.media {
            self.conn.execute(
                "INSERT INTO media_items (project_id, path, name, duration, width, height, frame_rate) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![project_id, item.path, item.name, item.duration, item.width, item.height, item.frame_rate],
            )?;
        }
        Ok(())
    }
}
