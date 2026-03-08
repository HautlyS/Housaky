use super::traits::{Tool, ToolResult};
use crate::security::SecurityPolicy;
use crate::util::expand_path;
use async_trait::async_trait;
use rusqlite::params;
use serde_json::json;
use std::sync::Arc;

pub struct LucidDbTool {
    security: Arc<SecurityPolicy>,
}

impl LucidDbTool {
    pub fn new(security: Arc<SecurityPolicy>) -> Self {
        Self { security }
    }
}

#[async_trait]
impl Tool for LucidDbTool {
    fn name(&self) -> &str {
        "lucid_db"
    }

    fn description(&self) -> &str {
        "Direct access to Lucid memory database for advanced queries and management. Query memories, entities, and knowledge graph."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["query", "stats", "search", "list_entities", "get_entity", "create_entity", "link_entities"],
                    "description": "Action to perform on Lucid database"
                },
                "query": {
                    "type": "string",
                    "description": "SQL query to execute (for query action)"
                },
                "search_term": {
                    "type": "string",
                    "description": "Search term (for search action)"
                },
                "memory_type": {
                    "type": "string",
                    "description": "Filter by memory type (conversation, semantic, episodic, etc.)"
                },
                "entity_id": {
                    "type": "string",
                    "description": "Entity ID (for get_entity action)"
                },
                "entity_name": {
                    "type": "string",
                    "description": "Entity name (for create_entity action)"
                },
                "entity_type": {
                    "type": "string",
                    "description": "Entity type (person, concept, project, etc.)"
                },
                "source_id": {
                    "type": "string",
                    "description": "Source entity ID (for link_entities)"
                },
                "target_id": {
                    "type": "string",
                    "description": "Target entity ID (for link_entities)"
                },
                "relation_type": {
                    "type": "string",
                    "description": "Relation type (for link_entities)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum results to return",
                    "default": 50
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        let db_path = expand_path("~/.housaky/workspace/.housaky/lucid.db");

        if !db_path.exists() {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Lucid database not found. Initialize memory first.".into()),
            });
        }

        if !self.security.can_act() && action != "query" && action != "stats" && action != "search" 
            && action != "list_entities" && action != "get_entity" {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some("Action blocked: autonomy is read-only".into()),
            });
        }

        match action {
            "query" => self.execute_query(&db_path, &args).await,
            "stats" => self.get_stats(&db_path).await,
            "search" => self.search_memories(&db_path, &args).await,
            "list_entities" => self.list_entities(&db_path, &args).await,
            "get_entity" => self.get_entity(&db_path, &args).await,
            "create_entity" => self.create_entity(&db_path, &args).await,
            "link_entities" => self.link_entities(&db_path, &args).await,
            _ => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown action: {}", action)),
            }),
        }
    }
}

impl LucidDbTool {
    async fn execute_query(
        &self,
        db_path: &std::path::Path,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'query' parameter"))?;

        let conn = rusqlite::Connection::open(db_path)?;

        if query.trim().to_uppercase().starts_with("SELECT") {
            let mut stmt = conn.prepare(query)?;
            let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

            let rows: Vec<Vec<serde_json::Value>> = stmt
                .query_map([], |row| {
                    let mut values = Vec::new();
                    for i in 0..columns.len() {
                        let value: rusqlite::types::Value = row.get(i)?;
                        values.push(self::value_to_json(value));
                    }
                    Ok(values)
                })?
                .collect::<Result<Vec<_>, _>>()?;

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
        } else {
            let affected = conn.execute(query, [])?;
            Ok(ToolResult {
                success: true,
                output: format!("Query executed. Rows affected: {}", affected),
                error: None,
            })
        }
    }

    async fn get_stats(&self, db_path: &std::path::Path) -> anyhow::Result<ToolResult> {
        let conn = rusqlite::Connection::open(db_path)?;

        let memory_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM memories",
            [],
            |row| row.get(0),
        )?;

        let entity_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entities",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let relation_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM relations",
            [],
            |row| row.get(0),
        ).unwrap_or(0);

        let db_size = std::fs::metadata(db_path)?.len();

        let type_counts: Vec<(String, i64)> = conn
            .prepare("SELECT memory_type, COUNT(*) as cnt FROM memories GROUP BY memory_type")?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let type_breakdown: serde_json::Value = type_counts
            .iter()
            .map(|(t, c)| (t.clone(), serde_json::json!(c)))
            .collect();

        let stats = serde_json::json!({
            "total_memories": memory_count,
            "total_entities": entity_count,
            "total_relations": relation_count,
            "db_size_bytes": db_size,
            "db_size_mb": db_size as f64 / 1_048_576.0,
            "by_type": type_breakdown
        });

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&stats)?,
            error: None,
        })
    }

    async fn search_memories(
        &self,
        db_path: &std::path::Path,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let search_term = args
            .get("search_term")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'search_term' parameter"))?;

        let memory_type = args.get("memory_type").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as i32;

        let conn = rusqlite::Connection::open(db_path)?;

        let sql = if let Some(_mt) = memory_type {
            format!(
                "SELECT id, memory_type, content, created_at FROM memories 
                 WHERE memory_type = ? AND content LIKE ? 
                 ORDER BY created_at DESC LIMIT ?"
            )
        } else {
            format!(
                "SELECT id, memory_type, content, created_at FROM memories 
                 WHERE content LIKE ? 
                 ORDER BY created_at DESC LIMIT ?"
            )
        };

        let pattern = format!("%{}%", search_term);

        let results: Vec<serde_json::Value> = if let Some(mt) = memory_type {
            conn.prepare(&sql)?
                .query_map(params![mt, &pattern, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "type": row.get::<_, String>(1)?,
                        "content": row.get::<_, String>(2)?,
                        "created_at": row.get::<_, String>(3)?
                    }))
                })?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            conn.prepare(&sql)?
                .query_map(params![&pattern, limit], |row| {
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "type": row.get::<_, String>(1)?,
                        "content": row.get::<_, String>(2)?,
                        "created_at": row.get::<_, String>(3)?
                    }))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&serde_json::json!({
                "query": search_term,
                "results": results,
                "count": results.len()
            }))?,
            error: None,
        })
    }

    async fn list_entities(
        &self,
        db_path: &std::path::Path,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let entity_type = args.get("entity_type").and_then(|v| v.as_str());
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as i32;

        let conn = rusqlite::Connection::open(db_path)?;

        let sql = if entity_type.is_some() {
            "SELECT id, name, entity_type, properties, created_at FROM entities WHERE entity_type = ? LIMIT ?"
        } else {
            "SELECT id, name, entity_type, properties, created_at FROM entities LIMIT ?"
        };

        let entities: Vec<serde_json::Value> = if let Some(et) = entity_type {
            conn.prepare(sql)?
                .query_map(params![et, limit], |row| {
                    let props: String = row.get(3)?;
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "type": row.get::<_, String>(2)?,
                        "properties": props,
                        "created_at": row.get::<_, String>(4)?
                    }))
                })?
                .collect::<Result<Vec<_>, _>>()?
        } else {
            conn.prepare(sql)?
                .query_map(params![limit], |row| {
                    let props: String = row.get(3)?;
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "type": row.get::<_, String>(2)?,
                        "properties": props,
                        "created_at": row.get::<_, String>(4)?
                    }))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&serde_json::json!({
                "entities": entities,
                "count": entities.len()
            }))?,
            error: None,
        })
    }

    async fn get_entity(
        &self,
        db_path: &std::path::Path,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let entity_id = args
            .get("entity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'entity_id' parameter"))?;

        let conn = rusqlite::Connection::open(db_path)?;

        let entity: Option<serde_json::Value> = conn
            .query_row(
                "SELECT id, name, entity_type, properties, created_at FROM entities WHERE id = ?",
                params![entity_id],
                |row| {
                    let props: String = row.get(3)?;
                    Ok(serde_json::json!({
                        "id": row.get::<_, String>(0)?,
                        "name": row.get::<_, String>(1)?,
                        "type": row.get::<_, String>(2)?,
                        "properties": props,
                        "created_at": row.get::<_, String>(4)?
                    }))
                },
            )
            .ok();

        if let Some(e) = entity {
            let relations: Vec<serde_json::Value> = conn
                .prepare(
                    "SELECT r.relation_type, e.id, e.name, e.entity_type 
                     FROM relations r 
                     JOIN entities e ON r.target_id = e.id 
                     WHERE r.source_id = ?"
                )?
                .query_map(params![entity_id], |row| {
                    Ok(serde_json::json!({
                        "relation": row.get::<_, String>(0)?,
                        "target_id": row.get::<_, String>(1)?,
                        "target_name": row.get::<_, String>(2)?,
                        "target_type": row.get::<_, String>(3)?
                    }))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(ToolResult {
                success: true,
                output: serde_json::to_string_pretty(&serde_json::json!({
                    "entity": e,
                    "relations": relations
                }))?,
                error: None,
            })
        } else {
            Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Entity '{}' not found", entity_id)),
            })
        }
    }

    async fn create_entity(
        &self,
        db_path: &std::path::Path,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let name = args
            .get("entity_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'entity_name' parameter"))?;

        let entity_type = args
            .get("entity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("concept");

        let properties = args.get("properties").cloned().unwrap_or(serde_json::json!({}));

        let conn = rusqlite::Connection::open(db_path)?;
        let entity_id = format!("entity_{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO entities (id, name, entity_type, properties, created_at) VALUES (?, ?, ?, ?, ?)",
            params![&entity_id, name, entity_type, properties.to_string(), &now],
        )?;

        Ok(ToolResult {
            success: true,
            output: format!("Entity '{}' created with ID: {}", name, entity_id),
            error: None,
        })
    }

    async fn link_entities(
        &self,
        db_path: &std::path::Path,
        args: &serde_json::Value,
    ) -> anyhow::Result<ToolResult> {
        let source_id = args
            .get("source_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'source_id' parameter"))?;

        let target_id = args
            .get("target_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'target_id' parameter"))?;

        let relation_type = args
            .get("relation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("related_to");

        let conn = rusqlite::Connection::open(db_path)?;
        let relation_id = format!("rel_{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO relations (id, source_id, target_id, relation_type, created_at) VALUES (?, ?, ?, ?, ?)",
            params![&relation_id, source_id, target_id, relation_type, &now],
        )?;

        Ok(ToolResult {
            success: true,
            output: format!("Relation '{}' created: {} -> {}", relation_type, source_id, target_id),
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
