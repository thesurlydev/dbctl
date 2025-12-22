use serde::{Deserialize, Serialize};

/// A single query tab
#[derive(Clone)]
pub struct QueryTab {
    pub id: usize,
    pub name: String,
    pub query: String,
    pub result: String,
    pub database: String,
    pub is_executing: bool,
}

impl QueryTab {
    pub fn new(id: usize, database: String) -> Self {
        Self {
            id,
            name: format!("Query {}", id),
            query: String::new(),
            result: String::new(),
            database,
            is_executing: false,
        }
    }
}

/// Query history entry
#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub query: String,
    pub database: String,
    pub timestamp: u64,
}

/// Saved query file info
#[derive(Clone)]
pub struct SavedQuery {
    pub name: String,
    pub query: String,
}

/// Enhanced SQL state with tabs, history, and saved queries
pub struct SqlState {
    pub tabs: Vec<QueryTab>,
    pub active_tab: usize,
    pub next_tab_id: usize,

    // History
    pub history: Vec<HistoryEntry>,
    pub history_index: Option<usize>,
    pub show_history_popup: bool,

    // Saved queries
    pub saved_queries: Vec<SavedQuery>,
    pub show_save_dialog: bool,
    pub save_query_name: String,
}

impl Default for SqlState {
    fn default() -> Self {
        let history = SqlState::load_history();
        let saved_queries = SqlState::load_saved_queries();
        Self {
            tabs: vec![QueryTab::new(1, "postgres".to_string())],
            active_tab: 0,
            next_tab_id: 2,
            history,
            history_index: None,
            show_history_popup: false,
            saved_queries,
            show_save_dialog: false,
            save_query_name: String::new(),
        }
    }
}

impl SqlState {
    fn history_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".dbctl")
            .join("query_history.json")
    }

    fn queries_dir() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".dbctl")
            .join("queries")
    }

    fn load_history() -> Vec<HistoryEntry> {
        let path = Self::history_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(history) = serde_json::from_str(&content) {
                    return history;
                }
            }
        }
        Vec::new()
    }

    fn save_history(&self) {
        let path = Self::history_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(&self.history) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn add_to_history(&mut self, query: &str, database: &str) {
        // Don't add empty or duplicate queries
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return;
        }
        if let Some(first) = self.history.first() {
            if first.query == trimmed && first.database == database {
                return;
            }
        }

        self.history.insert(
            0,
            HistoryEntry {
                query: trimmed.to_string(),
                database: database.to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            },
        );

        // Keep only 50 entries
        if self.history.len() > 50 {
            self.history.truncate(50);
        }
        self.save_history();
    }

    fn load_saved_queries() -> Vec<SavedQuery> {
        let dir = Self::queries_dir();
        if !dir.exists() {
            return Vec::new();
        }

        std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| {
                let path = entry.ok()?.path();
                if path.extension()?.to_str()? == "sql" {
                    let name = path.file_stem()?.to_string_lossy().to_string();
                    let query = std::fs::read_to_string(&path).ok()?;
                    Some(SavedQuery { name, query })
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn refresh_saved_queries(&mut self) {
        self.saved_queries = Self::load_saved_queries();
    }

    pub fn save_query(&mut self, name: &str, query: &str) -> Result<(), String> {
        // Sanitize name
        let sanitized: String = name
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if sanitized.is_empty() {
            return Err("Invalid query name".to_string());
        }

        let dir = Self::queries_dir();
        if let Err(e) = std::fs::create_dir_all(&dir) {
            return Err(format!("Failed to create directory: {}", e));
        }

        let path = dir.join(format!("{}.sql", sanitized));
        if let Err(e) = std::fs::write(&path, query) {
            return Err(format!("Failed to save query: {}", e));
        }

        self.refresh_saved_queries();
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_saved_query(&mut self, name: &str) -> Result<(), String> {
        let path = Self::queries_dir().join(format!("{}.sql", name));
        if let Err(e) = std::fs::remove_file(&path) {
            return Err(format!("Failed to delete query: {}", e));
        }
        self.refresh_saved_queries();
        Ok(())
    }

    pub fn current_tab(&self) -> &QueryTab {
        &self.tabs[self.active_tab]
    }

    pub fn current_tab_mut(&mut self) -> &mut QueryTab {
        &mut self.tabs[self.active_tab]
    }

    pub fn add_tab(&mut self, database: String) {
        if self.tabs.len() >= 10 {
            return; // Max 10 tabs
        }
        let tab = QueryTab::new(self.next_tab_id, database);
        self.next_tab_id += 1;
        self.tabs.push(tab);
        self.active_tab = self.tabs.len() - 1;
    }

    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 {
            return; // Keep at least one tab
        }
        self.tabs.remove(index);
        if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }
    }
}
