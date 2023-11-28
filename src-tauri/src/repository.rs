use async_trait::async_trait;
use rusqlite::params;

use tokio_rusqlite::Connection;

use crate::{error::SourceCmdGuiResult, python::Script};

#[async_trait]
pub trait Repository {
    async fn init(&self) -> SourceCmdGuiResult;
}
#[async_trait]
pub trait ScriptRepository {
    async fn add_script(&self, script: Script) -> SourceCmdGuiResult<Script>;
    async fn get_script(&self, id: i32) -> SourceCmdGuiResult<Script>;
    async fn update_script(&self, script: &Script) -> SourceCmdGuiResult;
    async fn delete_script(&self, id: i32) -> SourceCmdGuiResult;
    async fn get_scripts(&self) -> SourceCmdGuiResult<Vec<Script>>;
    async fn get_script_by_trigger(&self, trigger: &str) -> SourceCmdGuiResult<Option<Script>>;
}

pub struct SqliteRepository {
    conn: Connection,
}

impl SqliteRepository {
    pub async fn new(db_path: &str) -> SourceCmdGuiResult<Self> {
        let conn = Connection::open(db_path).await?;
        Ok(SqliteRepository { conn })
    }
}

#[async_trait]
impl Repository for SqliteRepository {
    async fn init(&self) -> SourceCmdGuiResult {
        self.conn
            .call(|conn| {
                conn.execute(
                    "CREATE TABLE IF NOT EXISTS scripts (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                code TEXT NOT NULL,
                trigger TEXT NOT NULL
            )",
                    [],
                )?;

                Ok(())
            })
            .await?;

        Ok(())
    }
}

#[async_trait]
impl ScriptRepository for SqliteRepository {
    async fn add_script(&self, script: Script) -> SourceCmdGuiResult<Script> {
        self.conn
            .call(move |conn| {
                let mut stmt =
                    conn.prepare("INSERT INTO scripts (name, code, trigger) VALUES (?1, ?2, ?3)")?;

                let id = stmt.insert(params![script.name, script.code, script.trigger])?;

                Ok(Script {
                    id: Some(id),
                    ..script
                })
            })
            .await
            .map_err(|e| e.into())
    }

    async fn get_script(&self, id: i32) -> SourceCmdGuiResult<Script> {
        self.conn
            .call(move |conn| {
                let mut stmt =
                    conn.prepare("SELECT id, name, code, trigger FROM scripts WHERE id = ?1")?;

                let script = stmt.query_row(params![id], |row| {
                    Ok(Script {
                        id: Some(row.get(0)?),
                        name: row.get(1)?,
                        code: row.get(2)?,
                        trigger: row.get(3)?,
                    })
                })?;

                Ok(script)
            })
            .await
            .map_err(|e| e.into())
    }

    async fn update_script(&self, script: &Script) -> SourceCmdGuiResult {
        let script = script.clone();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(
                    "UPDATE scripts SET name = ?1, code = ?2, trigger = ?3 WHERE id = ?4",
                )?;

                stmt.execute(params![script.name, script.code, script.trigger, script.id])?;

                Ok(())
            })
            .await
            .map_err(|e| e.into())
    }

    async fn delete_script(&self, id: i32) -> SourceCmdGuiResult {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("DELETE FROM scripts WHERE id = ?1")?;

                stmt.execute(params![id])?;

                Ok(())
            })
            .await
            .map_err(|e| e.into())
    }

    async fn get_scripts(&self) -> SourceCmdGuiResult<Vec<Script>> {
        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, code, trigger FROM scripts")?;

                let scripts = stmt.query_map([], |row| {
                    Ok(Script {
                        id: Some(row.get(0)?),
                        name: row.get(1)?,
                        code: row.get(2)?,
                        trigger: row.get(3)?,
                    })
                })?;

                let mut result = Vec::new();
                for script in scripts {
                    result.push(script?);
                }

                Ok(result)
            })
            .await
            .map_err(|e| e.into())
    }

    async fn get_script_by_trigger(&self, trigger: &str) -> SourceCmdGuiResult<Option<Script>> {
        let trigger = trigger.to_string();
        
        self.conn
            .call(move |conn| {
                let mut stmt =
                    conn.prepare("SELECT id, name, code, trigger FROM scripts WHERE trigger = ?1")?;

                let script = stmt.query_row(params![trigger], |row| {
                    Ok(Script {
                        id: Some(row.get(0)?),
                        name: row.get(1)?,
                        code: row.get(2)?,
                        trigger: row.get(3)?,
                    })
                })?;

                Ok(Some(script))
            })
            .await
            .map_err(|e| e.into())
    }
}
