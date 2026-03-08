use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

pub struct DatabaseTool {
    security: Arc<SecurityPolicy>,
}

impl DatabaseTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for DatabaseTool {
    fn name(&self) -> &str {
        "database"
    }

    fn description(&self) -> &str {
        "Execute SQL queries against SQLite databases. Supports SELECT, INSERT, UPDATE, DELETE, and DDL operations."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["query", "execute", "list_tables", "describe", "create_db"],
                    "description": "Action: query (SELECT), execute (INSERT/UPDATE/DELETE/DDL), list_tables, describe, create_db"
                },
                "database": {
                    "type": "string",
                    "description": "Database file path (relative to workspace, or :memory: for in-memory)"
                },
                "sql": {
                    "type": "string",
                    "description": "SQL query or statement to execute"
                },
                "params": {
                    "type": "array",
                    "description": "Parameters for parameterized queries",
                    "items": {}
                }
            },
            "required": ["action", "database"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        let database = args
            .get("database")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'database' parameter"))?;

        if !self.security.can_act() && action != "query" && action != "list_tables" && action != "describe" {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        let db_path = if database == ":memory:" {
            None
        } else {
            let path = expand_path(database);
            if !self.security.is_path_allowed(path.to_str().unwrap_or("")) {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Database path '{}' is outside allowed workspace", database)),
                });
            }
            Some(path)
        };

        match action {
            "query" => self.execute_query(db_path, &args).await,
            "execute" => self.execute_statement(db_path, &args).await,
            "list_tables" => self.list_tables(db_path).await,
            "describe" => self.describe_table(db_path, &args).await,
            "create_db" => self.create_database(db_path).await,
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            }),
        }
    }
}

impl DatabaseTool {
    async fn execute_query(
        &self,
        db_path: Option<std::path::PathBuf>,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let sql = args
            .get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'sql' parameter"))?;

        let params = args.get("params").and_then(|v| v.as_array()).cloned();

        let path = db_path.unwrap_or_else(|| std::path::PathBuf::from(":memory:"));
        let _path_str = path.to_string_lossy().to_string();
        let conn = rusqlite::Connection::open(&path)?;

        let mut stmt = conn.prepare(sql)?;
        let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let rows = if let Some(p) = params {
            let params: Vec<rusqlite::types::Value> = p
                .iter()
                .map(|v| match v {
                    serde_json::Value::Null => rusqlite::types::Value::Null,
                    serde_json::Value::Bool(b) => rusqlite::types::Value::Integer(*b as i64),
                    serde_json::Value::Number(n) => rusqlite::types::Value::Integer(n.as_i64().unwrap_or(0)),
                    serde_json::Value::String(s) => rusqlite::types::Value::Text(s.clone()),
                    _ => rusqlite::types::Value::Text(v.to_string()),
                })
                .collect();
            let params_refs: Vec<&rusqlite::types::Value> = params.iter().collect();
            stmt.query_map(rusqlite::params_from_iter(params_refs.iter()), |row| {
                let mut values = Vec::new();
                for i in 0..columns.len() {
                    let value: rusqlite::types::Value = row.get(i)?;
                    values.push(value_to_json(value));
                }
                Ok(values)
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], |row| {
                let mut values = Vec::new();
                for i in 0..columns.len() {
                    let value: rusqlite::types::Value = row.get(i)?;
                    values.push(value_to_json(value));
                }
                Ok(values)
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        let result = serde_json::json!({
            "columns": columns,
            "rows": rows,
            "row_count": rows.len()
        });

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&result)?,
            error: None,
        })
    }

    async fn execute_statement(
        &self,
        db_path: Option<std::path::PathBuf>,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let sql = args
            .get("sql")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'sql' parameter"))?;

        let path = db_path.unwrap_or_else(|| std::path::PathBuf::from(":memory:"));
        let conn = rusqlite::Connection::open(&path)?;

        let result = conn.execute(sql, [])?;

        Ok(ToolResult {
            success: true,
            output: format!("Statement executed. Rows affected: {}", result),
            error: None,
        })
    }

    async fn list_tables(&self, db_path: Option<std::path::PathBuf>) -> anyhow::Result<ToolResult> {
        let path = db_path.unwrap_or_else(|| std::path::PathBuf::from(":memory:"));
        let conn = rusqlite::Connection::open(&path)?;

        let mut stmt = conn.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )?;

        let tables: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ToolResult {
            success: true,
            output: format!("Tables:\n{}", tables.join("\n")),
            error: None,
        })
    }

    async fn describe_table(
        &self,
        db_path: Option<std::path::PathBuf>,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let table_name = args
            .get("table")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'table' parameter"))?;

        let path = db_path.unwrap_or_else(|| std::path::PathBuf::from(":memory:"));
        let conn = rusqlite::Connection::open(&path)?;

        let sql = format!("PRAGMA table_info({})", table_name);
        let mut stmt = conn.prepare(&sql)?;

        let columns: Vec<serde_json::Value> = stmt
            .query_map([], |row| {
                Ok(serde_json::json!({
                    "cid": row.get::<_, i64>(0)?,
                    "name": row.get::<_, String>(1)?,
                    "type": row.get::<_, String>(2)?,
                    "notnull": row.get::<_, i64>(3)?,
                    "default": row.get::<_, Option<String>>(4)?,
                    "pk": row.get::<_, i64>(5)?
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&serde_json::json!({
                "table": table_name,
                "columns": columns
            }))?,
            error: None,
        })
    }

    async fn create_database(&self, db_path: Option<std::path::PathBuf>) -> anyhow::Result<ToolResult> {
        let path = db_path.ok_or_else(|| anyhow::anyhow!("Cannot create in-memory database"))?;

        if path.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Database already exists".into()),
            });
        }

        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let _conn = rusqlite::Connection::open(&path)?;

        Ok(ToolResult {
            success: true,
            output: format!("Database created at {}", path.display()),
            error: None,
        })
    }
}

fn value_to_json(value: rusqlite::types::Value) -> serde_json::Value {
    use rusqlite::types::Value;
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Integer(i) => serde_json::json!(i),
        Value::Real(f) => serde_json::json!(f),
        Value::Text(s) => serde_json::json!(s),
        Value::Blob(b) => serde_json::json!(format!("<blob:{} bytes>", b.len())),
    }
}
