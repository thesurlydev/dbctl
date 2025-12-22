use eframe::egui;
use std::collections::HashMap;

/// A complete schema model for editing
#[derive(Debug, Clone, Default)]
pub struct SchemaModel {
    pub tables: HashMap<String, TableModel>,
}

/// A table with its columns, constraints, and indexes
#[derive(Debug, Clone)]
pub struct TableModel {
    pub name: String,
    pub columns: Vec<ColumnModel>,
    pub constraints: Vec<ConstraintModel>,
    pub indexes: Vec<IndexModel>,

    // Canvas positioning
    pub position: egui::Pos2,
    pub collapsed: bool,
}

impl Default for TableModel {
    fn default() -> Self {
        Self {
            name: String::new(),
            columns: Vec::new(),
            constraints: Vec::new(),
            indexes: Vec::new(),
            position: egui::Pos2::ZERO,
            collapsed: false,
        }
    }
}

impl TableModel {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    /// Get the primary key columns
    pub fn primary_key_columns(&self) -> Vec<&str> {
        self.columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| c.name.as_str())
            .collect()
    }

    /// Get foreign key constraints
    pub fn foreign_keys(&self) -> Vec<&ConstraintModel> {
        self.constraints
            .iter()
            .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct ColumnModel {
    pub name: String,
    pub data_type: String,
    pub is_nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

impl Default for ColumnModel {
    fn default() -> Self {
        Self {
            name: String::new(),
            data_type: "text".to_string(),
            is_nullable: true,
            default_value: None,
            is_primary_key: false,
        }
    }
}

impl ColumnModel {
    pub fn new(name: &str, data_type: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: data_type.to_string(),
            ..Default::default()
        }
    }

    pub fn primary_key(name: &str, data_type: &str) -> Self {
        Self {
            name: name.to_string(),
            data_type: data_type.to_string(),
            is_nullable: false,
            is_primary_key: true,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConstraintModel {
    pub name: String,
    pub constraint_type: ConstraintType,
    pub columns: Vec<String>,
    // For foreign keys
    pub foreign_table: Option<String>,
    pub foreign_columns: Option<Vec<String>>,
    pub on_delete: Option<FkAction>,
    pub on_update: Option<FkAction>,
}

impl Default for ConstraintModel {
    fn default() -> Self {
        Self {
            name: String::new(),
            constraint_type: ConstraintType::Check,
            columns: Vec::new(),
            foreign_table: None,
            foreign_columns: None,
            on_delete: None,
            on_update: None,
        }
    }
}

impl ConstraintModel {
    /// Create a foreign key constraint
    pub fn foreign_key(
        name: &str,
        column: &str,
        ref_table: &str,
        ref_column: &str,
        on_delete: FkAction,
    ) -> Self {
        Self {
            name: name.to_string(),
            constraint_type: ConstraintType::ForeignKey,
            columns: vec![column.to_string()],
            foreign_table: Some(ref_table.to_string()),
            foreign_columns: Some(vec![ref_column.to_string()]),
            on_delete: Some(on_delete),
            on_update: Some(FkAction::NoAction),
        }
    }

    /// Create a unique constraint
    pub fn unique(name: &str, columns: Vec<String>) -> Self {
        Self {
            name: name.to_string(),
            constraint_type: ConstraintType::Unique,
            columns,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum ConstraintType {
    PrimaryKey,
    ForeignKey,
    Unique,
    #[default]
    Check,
}

impl std::fmt::Display for ConstraintType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstraintType::PrimaryKey => write!(f, "PRIMARY KEY"),
            ConstraintType::ForeignKey => write!(f, "FOREIGN KEY"),
            ConstraintType::Unique => write!(f, "UNIQUE"),
            ConstraintType::Check => write!(f, "CHECK"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Copy)]
pub enum FkAction {
    #[default]
    NoAction,
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
}

impl std::fmt::Display for FkAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FkAction::NoAction => write!(f, "NO ACTION"),
            FkAction::Restrict => write!(f, "RESTRICT"),
            FkAction::Cascade => write!(f, "CASCADE"),
            FkAction::SetNull => write!(f, "SET NULL"),
            FkAction::SetDefault => write!(f, "SET DEFAULT"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct IndexModel {
    pub name: String,
    pub columns: Vec<String>,
    pub is_unique: bool,
    pub index_type: String, // btree, hash, gin, gist, etc.
    pub is_primary: bool,
}

impl Default for IndexModel {
    fn default() -> Self {
        Self {
            name: String::new(),
            columns: Vec::new(),
            is_unique: false,
            index_type: "btree".to_string(),
            is_primary: false,
        }
    }
}

impl IndexModel {
    pub fn new(name: &str, columns: Vec<String>, is_unique: bool) -> Self {
        Self {
            name: name.to_string(),
            columns,
            is_unique,
            ..Default::default()
        }
    }
}
