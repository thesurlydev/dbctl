//! Claude API client for schema generation

use super::{ChatMessage, MessageRole, SchemaChangeProposal, TableModification};
use crate::schema_designer::{
    ColumnModel, ConstraintModel, ConstraintType, FkAction, IndexModel, SchemaModel, TableModel,
};
use eframe::egui;
use serde::{Deserialize, Serialize};

const ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";
const MODEL: &str = "claude-sonnet-4-20250514";
const MAX_TOKENS: u32 = 4096;

/// Request body for Claude API
#[derive(Serialize)]
struct ClaudeRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<ClaudeMessage>,
}

#[derive(Serialize)]
struct ClaudeMessage {
    role: String,
    content: String,
}

/// Response from Claude API
#[derive(Deserialize)]
struct ClaudeResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

/// Generate system prompt with current schema context
fn build_system_prompt(current_schema: &SchemaModel) -> String {
    // Serialize schema to a readable format
    let schema_description = if current_schema.tables.is_empty() {
        "No tables currently exist.".to_string()
    } else {
        let mut desc = String::new();
        for (name, table) in &current_schema.tables {
            desc.push_str(&format!("\nTable: {}\n", name));
            desc.push_str("  Columns:\n");
            for col in &table.columns {
                let pk = if col.is_primary_key { " [PK]" } else { "" };
                let nullable = if col.is_nullable { "" } else { " NOT NULL" };
                let default = col
                    .default_value
                    .as_ref()
                    .map(|d| format!(" DEFAULT {}", d))
                    .unwrap_or_default();
                desc.push_str(&format!(
                    "    - {}: {}{}{}{}\n",
                    col.name, col.data_type, pk, nullable, default
                ));
            }

            let fks: Vec<_> = table
                .constraints
                .iter()
                .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
                .collect();
            if !fks.is_empty() {
                desc.push_str("  Foreign Keys:\n");
                for fk in fks {
                    if let (Some(ref_table), Some(ref_cols)) =
                        (&fk.foreign_table, &fk.foreign_columns)
                    {
                        desc.push_str(&format!(
                            "    - {} -> {}.{}\n",
                            fk.columns.join(", "),
                            ref_table,
                            ref_cols.join(", ")
                        ));
                    }
                }
            }

            if !table.indexes.is_empty() {
                desc.push_str("  Indexes:\n");
                for idx in &table.indexes {
                    let unique = if idx.is_unique { "UNIQUE " } else { "" };
                    desc.push_str(&format!(
                        "    - {}{}({})\n",
                        unique,
                        idx.name,
                        idx.columns.join(", ")
                    ));
                }
            }
        }
        desc
    };

    format!(
        r#"You are a PostgreSQL database schema designer assistant integrated into a visual schema designer tool. You help users design and modify database schemas through natural language conversation.

CURRENT SCHEMA:
{}

INSTRUCTIONS:
1. When the user describes schema changes, respond with a clear explanation of what you'll do.
2. Include a JSON block with the schema changes in this exact format. Use ```json code blocks.
3. Only include sections that have changes (omit empty arrays).
4. Use appropriate PostgreSQL data types (bigserial, varchar, text, integer, boolean, timestamptz, etc.).
5. Follow best practices: use bigserial for PKs, include created_at/updated_at timestamps, proper foreign keys.
6. If the user's request is unclear, ask clarifying questions before proposing changes.

JSON FORMAT for schema changes:
```json
{{
  "description": "Brief description of changes",
  "tables_to_add": [
    {{
      "name": "table_name",
      "columns": [
        {{"name": "id", "data_type": "bigserial", "is_primary_key": true, "is_nullable": false}},
        {{"name": "name", "data_type": "varchar(255)", "is_nullable": false}},
        {{"name": "created_at", "data_type": "timestamptz", "is_nullable": false, "default_value": "now()"}}
      ],
      "indexes": [
        {{"name": "idx_table_column", "columns": ["column_name"], "is_unique": false}}
      ],
      "foreign_keys": [
        {{"name": "fk_table_ref", "columns": ["ref_id"], "references_table": "other_table", "references_columns": ["id"], "on_delete": "CASCADE"}}
      ]
    }}
  ],
  "tables_to_modify": [
    {{
      "table": "existing_table",
      "add_columns": [{{"name": "new_col", "data_type": "text", "is_nullable": true}}],
      "remove_columns": ["old_col"]
    }}
  ],
  "tables_to_remove": ["old_table"]
}}
```

Remember: Be concise but thorough. Suggest best practices when relevant."#,
        schema_description
    )
}

/// Call Claude API to generate schema changes
pub async fn generate_schema_changes(
    api_key: &str,
    current_schema: &SchemaModel,
    conversation_history: &[ChatMessage],
    new_message: &str,
) -> Result<(String, Option<SchemaChangeProposal>), String> {
    let client = reqwest::Client::new();

    // Build messages from history
    let mut messages: Vec<ClaudeMessage> = conversation_history
        .iter()
        .map(|msg| ClaudeMessage {
            role: match msg.role {
                MessageRole::User => "user".to_string(),
                MessageRole::Assistant => "assistant".to_string(),
            },
            content: msg.content.clone(),
        })
        .collect();

    // Add the new message
    messages.push(ClaudeMessage {
        role: "user".to_string(),
        content: new_message.to_string(),
    });

    let request = ClaudeRequest {
        model: MODEL.to_string(),
        max_tokens: MAX_TOKENS,
        system: build_system_prompt(current_schema),
        messages,
    };

    let response = client
        .post(ANTHROPIC_API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("API error ({}): {}", status, error_text));
    }

    let claude_response: ClaudeResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    let response_text = claude_response
        .content
        .into_iter()
        .filter_map(|block| {
            if block.block_type == "text" {
                block.text
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("");

    // Parse schema changes from response
    let schema_changes = parse_schema_changes(&response_text);

    Ok((response_text, schema_changes))
}

/// Parse JSON schema changes from Claude's response
fn parse_schema_changes(response: &str) -> Option<SchemaChangeProposal> {
    // Find JSON block in response
    let json_start = response.find("```json")?;
    let json_content_start = json_start + 7;
    let json_end = response[json_content_start..].find("```")?;
    let json_str = &response[json_content_start..json_content_start + json_end].trim();

    // Parse JSON
    let json: serde_json::Value = serde_json::from_str(json_str).ok()?;

    let mut proposal = SchemaChangeProposal::default();

    // Parse description
    proposal.description = json
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("Schema changes")
        .to_string();

    // Parse tables to add
    if let Some(tables) = json.get("tables_to_add").and_then(|v| v.as_array()) {
        for table_json in tables {
            if let Some(table) = parse_table_from_json(table_json) {
                proposal.tables_to_add.push(table);
            }
        }
    }

    // Parse tables to modify
    if let Some(mods) = json.get("tables_to_modify").and_then(|v| v.as_array()) {
        for mod_json in mods {
            if let Some(modification) = parse_table_modification(mod_json) {
                proposal.tables_to_modify.push(modification);
            }
        }
    }

    // Parse tables to remove
    if let Some(removes) = json.get("tables_to_remove").and_then(|v| v.as_array()) {
        for name in removes {
            if let Some(s) = name.as_str() {
                proposal.tables_to_remove.push(s.to_string());
            }
        }
    }

    // Only return if there are actual changes
    if proposal.tables_to_add.is_empty()
        && proposal.tables_to_modify.is_empty()
        && proposal.tables_to_remove.is_empty()
        && proposal.constraints_to_add.is_empty()
        && proposal.indexes_to_add.is_empty()
    {
        return None;
    }

    Some(proposal)
}

fn parse_table_from_json(json: &serde_json::Value) -> Option<TableModel> {
    let name = json.get("name")?.as_str()?.to_string();

    let mut columns = Vec::new();
    if let Some(cols) = json.get("columns").and_then(|v| v.as_array()) {
        for col_json in cols {
            if let Some(col) = parse_column_from_json(col_json) {
                columns.push(col);
            }
        }
    }

    let mut constraints = Vec::new();
    if let Some(fks) = json.get("foreign_keys").and_then(|v| v.as_array()) {
        for fk_json in fks {
            if let Some(fk) = parse_foreign_key_from_json(fk_json) {
                constraints.push(fk);
            }
        }
    }

    let mut indexes = Vec::new();
    if let Some(idxs) = json.get("indexes").and_then(|v| v.as_array()) {
        for idx_json in idxs {
            if let Some(idx) = parse_index_from_json(idx_json) {
                indexes.push(idx);
            }
        }
    }

    Some(TableModel {
        name,
        columns,
        constraints,
        indexes,
        position: egui::pos2(50.0, 50.0), // Will be auto-positioned
        collapsed: false,
    })
}

fn parse_column_from_json(json: &serde_json::Value) -> Option<ColumnModel> {
    let name = json.get("name")?.as_str()?.to_string();
    let data_type = json.get("data_type")?.as_str()?.to_string();
    let is_nullable = json
        .get("is_nullable")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let is_primary_key = json
        .get("is_primary_key")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let default_value = json
        .get("default_value")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Some(ColumnModel {
        name,
        data_type,
        is_nullable,
        default_value,
        is_primary_key,
    })
}

fn parse_foreign_key_from_json(json: &serde_json::Value) -> Option<ConstraintModel> {
    let name = json.get("name")?.as_str()?.to_string();
    let columns: Vec<String> = json
        .get("columns")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let ref_table = json.get("references_table")?.as_str()?.to_string();
    let ref_columns: Vec<String> = json
        .get("references_columns")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    let on_delete = json
        .get("on_delete")
        .and_then(|v| v.as_str())
        .map(|s| match s.to_uppercase().as_str() {
            "CASCADE" => FkAction::Cascade,
            "SET NULL" => FkAction::SetNull,
            "SET DEFAULT" => FkAction::SetDefault,
            "RESTRICT" => FkAction::Restrict,
            _ => FkAction::NoAction,
        })
        .unwrap_or(FkAction::NoAction);

    Some(ConstraintModel {
        name,
        constraint_type: ConstraintType::ForeignKey,
        columns,
        foreign_table: Some(ref_table),
        foreign_columns: Some(ref_columns),
        on_delete: Some(on_delete),
        on_update: Some(FkAction::NoAction),
    })
}

fn parse_index_from_json(json: &serde_json::Value) -> Option<IndexModel> {
    let name = json.get("name")?.as_str()?.to_string();
    let columns: Vec<String> = json
        .get("columns")?
        .as_array()?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let is_unique = json
        .get("is_unique")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Some(IndexModel {
        name,
        columns,
        is_unique,
        index_type: "btree".to_string(),
        is_primary: false,
    })
}

fn parse_table_modification(json: &serde_json::Value) -> Option<TableModification> {
    let table_name = json.get("table")?.as_str()?.to_string();

    let mut columns_to_add = Vec::new();
    if let Some(cols) = json.get("add_columns").and_then(|v| v.as_array()) {
        for col_json in cols {
            if let Some(col) = parse_column_from_json(col_json) {
                columns_to_add.push(col);
            }
        }
    }

    let mut columns_to_remove = Vec::new();
    if let Some(cols) = json.get("remove_columns").and_then(|v| v.as_array()) {
        for col in cols {
            if let Some(name) = col.as_str() {
                columns_to_remove.push(name.to_string());
            }
        }
    }

    Some(TableModification {
        table_name,
        columns_to_add,
        columns_to_modify: Vec::new(),
        columns_to_remove,
    })
}
