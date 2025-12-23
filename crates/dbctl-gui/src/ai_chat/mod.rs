//! AI Chat module for schema generation using Claude API

pub mod claude_client;

use crate::schema_designer::{ColumnModel, ConstraintModel, IndexModel, TableModel};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

/// Available Claude models
pub const AVAILABLE_MODELS: &[(&str, &str)] = &[
    ("claude-sonnet-4-20250514", "Claude Sonnet 4 (Recommended)"),
    ("claude-opus-4-20250514", "Claude Opus 4"),
    ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet"),
    ("claude-3-5-haiku-20241022", "Claude 3.5 Haiku (Fast)"),
];

/// Default values for AI configuration
pub const DEFAULT_MODEL: &str = "claude-sonnet-4-20250514";
pub const DEFAULT_MAX_TOKENS: u32 = 4096;

/// AI configuration persisted to disk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Anthropic API key
    #[serde(default)]
    pub api_key: String,
    /// Model to use for schema generation
    #[serde(default = "default_model")]
    pub model: String,
    /// Maximum tokens for response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

fn default_max_tokens() -> u32 {
    DEFAULT_MAX_TOKENS
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            model: DEFAULT_MODEL.to_string(),
            max_tokens: DEFAULT_MAX_TOKENS,
        }
    }
}

impl AiConfig {
    /// Get the config file path (~/.dbctl/ai-config.json)
    fn config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".dbctl").join("ai-config.json")
    }

    /// Load config from disk, or return default if not found
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(config) = serde_json::from_str(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    /// Save config to disk
    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;

        fs::write(&path, content).map_err(|e| format!("Failed to write config: {}", e))?;

        Ok(())
    }

    /// Check if API key is configured
    pub fn has_api_key(&self) -> bool {
        !self.api_key.is_empty()
    }

    /// Get display name for current model
    pub fn model_display_name(&self) -> &str {
        AVAILABLE_MODELS
            .iter()
            .find(|(id, _)| *id == self.model)
            .map(|(_, name)| *name)
            .unwrap_or(&self.model)
    }
}

/// Chat message role
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
}

/// A single chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip)]
    pub timestamp: Option<SystemTime>,
}

impl ChatMessage {
    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            timestamp: Some(SystemTime::now()),
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            timestamp: Some(SystemTime::now()),
        }
    }
}

/// Proposed schema changes from AI
#[derive(Debug, Clone, Default)]
pub struct SchemaChangeProposal {
    pub description: String,
    pub tables_to_add: Vec<TableModel>,
    pub tables_to_modify: Vec<TableModification>,
    pub tables_to_remove: Vec<String>,
    pub constraints_to_add: Vec<(String, ConstraintModel)>,
    pub indexes_to_add: Vec<(String, IndexModel)>,
}

/// Modification to an existing table
#[derive(Debug, Clone)]
pub struct TableModification {
    pub table_name: String,
    pub columns_to_add: Vec<ColumnModel>,
    pub columns_to_modify: Vec<ColumnModel>,
    pub columns_to_remove: Vec<String>,
}

/// State for the AI chat sidebar
#[derive(Default)]
pub struct AiChatState {
    /// Sidebar visibility
    pub visible: bool,

    /// Sidebar width
    pub width: f32,

    /// Chat history (session only, not persisted)
    pub messages: Vec<ChatMessage>,

    /// Current input text
    pub input_text: String,

    /// Loading state
    pub is_loading: bool,

    /// Error message
    pub error_message: Option<String>,

    /// Pending schema changes from AI
    pub pending_changes: Option<SchemaChangeProposal>,
}

impl AiChatState {
    pub fn new() -> Self {
        Self {
            visible: false,
            width: 350.0,
            messages: Vec::new(),
            input_text: String::new(),
            is_loading: false,
            error_message: None,
            pending_changes: None,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn add_user_message(&mut self, content: &str) {
        self.messages.push(ChatMessage::user(content));
    }

    pub fn add_assistant_message(&mut self, content: &str) {
        self.messages.push(ChatMessage::assistant(content));
    }

    pub fn clear_history(&mut self) {
        self.messages.clear();
        self.pending_changes = None;
        self.error_message = None;
    }
}
