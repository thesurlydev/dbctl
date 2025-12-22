use chrono::{DateTime, Local};
use eframe::egui;
use super::models::{ColumnModel, ConstraintModel, IndexModel, TableModel};

/// Actions for undo/redo support
#[derive(Debug, Clone)]
pub enum SchemaAction {
    // Table operations
    CreateTable(TableModel),
    DropTable(TableModel),
    RenameTable { old_name: String, new_name: String },
    MoveTable { table: String, old_pos: egui::Pos2, new_pos: egui::Pos2 },

    // Column operations
    AddColumn { table: String, column: ColumnModel },
    DropColumn { table: String, column: ColumnModel },
    AlterColumn { table: String, old: ColumnModel, new: ColumnModel },
    RenameColumn { table: String, old_name: String, new_name: String },

    // Index operations
    CreateIndex { table: String, index: IndexModel },
    DropIndex { table: String, index: IndexModel },

    // Constraint operations
    AddConstraint { table: String, constraint: ConstraintModel },
    DropConstraint { table: String, constraint: ConstraintModel },

    // Compound actions (for grouping)
    Batch(Vec<SchemaAction>),
}

impl SchemaAction {
    /// Get a human-readable description of the action
    pub fn description(&self) -> String {
        match self {
            SchemaAction::CreateTable(t) => format!("Create table '{}'", t.name),
            SchemaAction::DropTable(t) => format!("Drop table '{}'", t.name),
            SchemaAction::RenameTable { old_name, new_name } => {
                format!("Rename table '{}' to '{}'", old_name, new_name)
            }
            SchemaAction::MoveTable { table, .. } => format!("Move table '{}'", table),
            SchemaAction::AddColumn { table, column } => {
                format!("Add column '{}.{}'", table, column.name)
            }
            SchemaAction::DropColumn { table, column } => {
                format!("Drop column '{}.{}'", table, column.name)
            }
            SchemaAction::AlterColumn { table, old, new } => {
                if old.name != new.name {
                    format!("Rename column '{}.{}' to '{}'", table, old.name, new.name)
                } else {
                    format!("Alter column '{}.{}'", table, old.name)
                }
            }
            SchemaAction::RenameColumn { table, old_name, new_name } => {
                format!("Rename column '{}.{}' to '{}'", table, old_name, new_name)
            }
            SchemaAction::CreateIndex { table, index } => {
                format!("Create index '{}' on '{}'", index.name, table)
            }
            SchemaAction::DropIndex { table, index } => {
                format!("Drop index '{}' from '{}'", index.name, table)
            }
            SchemaAction::AddConstraint { table, constraint } => {
                format!("Add {} constraint '{}' on '{}'", constraint.constraint_type, constraint.name, table)
            }
            SchemaAction::DropConstraint { table, constraint } => {
                format!("Drop constraint '{}' from '{}'", constraint.name, table)
            }
            SchemaAction::Batch(actions) => {
                if actions.len() == 1 {
                    actions[0].description()
                } else {
                    format!("{} changes", actions.len())
                }
            }
        }
    }
}

/// An action with a timestamp for the audit log
#[derive(Debug, Clone)]
pub struct TimestampedAction {
    pub action: SchemaAction,
    pub timestamp: DateTime<Local>,
}

impl TimestampedAction {
    pub fn new(action: SchemaAction) -> Self {
        Self {
            action,
            timestamp: Local::now(),
        }
    }

    /// Get the action description
    pub fn description(&self) -> String {
        self.action.description()
    }

    /// Format the timestamp for display
    pub fn format_timestamp(&self) -> String {
        let now = Local::now();
        let duration = now.signed_duration_since(self.timestamp);

        if duration.num_seconds() < 60 {
            "Just now".to_string()
        } else if duration.num_minutes() < 60 {
            let mins = duration.num_minutes();
            if mins == 1 {
                "1 min ago".to_string()
            } else {
                format!("{} mins ago", mins)
            }
        } else if duration.num_hours() < 24 {
            self.timestamp.format("%l:%M %p").to_string().trim().to_string()
        } else {
            self.timestamp.format("%b %d, %l:%M %p").to_string().trim().to_string()
        }
    }
}
