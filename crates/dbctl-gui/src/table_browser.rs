use std::collections::{HashMap, HashSet};

use crate::state::ExportFormat;

/// State for the table browser tab
#[derive(Default)]
pub struct TableBrowserState {
    // Navigation
    pub selected_database: Option<String>,
    pub schemas: Vec<dbctl::SchemaInfo>,
    pub selected_schema: Option<String>,
    pub tables: Vec<dbctl::TableInfo>,
    pub selected_table: Option<String>,

    // Schema tree expansion state
    pub expanded_schemas: HashSet<String>,

    // Table structure
    pub table_structure: Option<dbctl::TableStructure>,

    // Data browsing
    pub table_data: Option<dbctl::TableDataResult>,
    pub current_page: usize,
    pub page_size: usize,
    pub total_rows: i64,

    // Inline editing
    #[allow(dead_code)]
    pub editing_row: Option<usize>,
    #[allow(dead_code)]
    pub editing_column: Option<String>,
    #[allow(dead_code)]
    pub edit_buffer: String,

    // New row insertion
    pub show_insert_dialog: bool,
    pub new_row_values: HashMap<String, String>,

    // Export
    #[allow(dead_code)]
    pub show_export_dialog: bool,
    pub export_format: ExportFormat,
    #[allow(dead_code)]
    pub export_path: String,

    // Delete confirmation
    pub show_delete_dialog: bool,
    pub pending_delete_pk: Option<String>,

    // Loading states
    pub loading_schemas: bool,
    pub loading_tables: bool,
    pub loading_structure: bool,
    pub loading_data: bool,
}

impl TableBrowserState {
    pub fn new() -> Self {
        Self {
            page_size: 50,
            export_format: ExportFormat::Csv,
            ..Default::default()
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}
