//! AI Chat module for schema generation using Claude API

pub mod claude_client;

use crate::schema_designer::{ColumnModel, ConstraintModel, IndexModel, TableModel};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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

/// API key management using system keychain
pub struct ApiKeyManager;

impl ApiKeyManager {
    const SERVICE: &'static str = "dbctl-gui";
    const USERNAME: &'static str = "anthropic_api_key";

    /// Get stored API key
    pub fn get_api_key() -> Option<String> {
        keyring::Entry::new(Self::SERVICE, Self::USERNAME)
            .ok()
            .and_then(|entry| entry.get_password().ok())
    }

    /// Store API key securely
    pub fn set_api_key(key: &str) -> Result<(), keyring::Error> {
        let entry = keyring::Entry::new(Self::SERVICE, Self::USERNAME)?;
        entry.set_password(key)
    }

    /// Delete stored API key
    #[allow(dead_code)]
    pub fn delete_api_key() -> Result<(), keyring::Error> {
        let entry = keyring::Entry::new(Self::SERVICE, Self::USERNAME)?;
        entry.delete_credential()
    }

    /// Check if API key is configured
    pub fn has_api_key() -> bool {
        Self::get_api_key().is_some()
    }
}
