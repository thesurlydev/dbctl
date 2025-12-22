mod actions;
pub mod canvas;
mod diff;
mod models;
pub mod templates;

pub use actions::{SchemaAction, TimestampedAction};
pub use canvas::CanvasState;
pub use diff::{generate_ddl_sql, generate_migration_sql};
pub use models::{ColumnModel, ConstraintModel, ConstraintType, FkAction, IndexModel, SchemaModel, TableModel};
pub use templates::{get_all_templates, TemplateCategory};

use std::collections::{HashMap, HashSet};

/// Main state for the schema designer tab
#[derive(Default)]
pub struct SchemaDesignerState {
    // Database context
    pub selected_database: Option<String>,
    pub selected_schema: String,

    // Original schema (from database)
    pub original_schema: SchemaModel,

    // Working schema (user modifications)
    pub working_schema: SchemaModel,

    // Canvas state
    pub canvas: CanvasState,

    // Visual column order (for drag-and-drop reordering without affecting schema)
    // Maps table_name -> ordered list of column indices
    pub visual_column_order: HashMap<String, Vec<usize>>,

    // Selection state
    pub selected_table: Option<String>,
    pub selected_column: Option<(String, String)>, // (table_name, column_name)
    pub selected_index: Option<(String, String)>,  // (table_name, index_name)

    // Edit modes
    pub renaming_target: Option<RenameTarget>,
    pub rename_buffer: String,

    // Dialogs
    pub show_new_table_dialog: bool,
    pub show_new_column_dialog: bool,
    pub show_new_index_dialog: bool,
    pub show_new_fk_dialog: bool,
    pub show_apply_migration_dialog: bool,
    pub show_edit_column_dialog: bool,

    // New table form data
    pub new_table_name: String,
    pub focus_new_table_name: bool,

    // New column form data
    pub new_column_name: String,
    pub focus_new_column_name: bool,
    pub new_column_type: String,
    pub new_column_size: String,
    pub new_column_nullable: bool,
    pub new_column_default: String,
    pub new_column_pk: bool,

    // Edit column form data
    pub edit_column_name: String,
    pub edit_column_type: String,
    pub edit_column_size: String,
    pub edit_column_nullable: bool,
    pub edit_column_default: String,

    // New index form data
    pub new_index_name: String,
    pub new_index_columns: HashSet<String>,
    pub new_index_unique: bool,

    // New FK form data
    pub new_fk_column: String,
    pub new_fk_ref_table: String,
    pub new_fk_ref_column: String,
    pub new_fk_on_delete: FkAction,

    // Migration preview
    pub pending_sql: Vec<String>,
    pub migration_error: Option<String>,

    // Undo/Redo with timestamps
    pub action_history: Vec<TimestampedAction>,
    pub action_index: usize,

    // History panel visibility
    pub history_panel_visible: bool,

    // Loading states
    pub loading_schema: bool,
    pub applying_migration: bool,

    // SQL preview panel expanded
    pub sql_preview_expanded: bool,

    // DDL panel expanded
    pub ddl_panel_expanded: bool,

    // Template selection state
    pub show_template_dialog: bool,
    pub template_search_query: String,
    pub selected_template: Option<String>,
    pub template_preview: Option<SchemaModel>,
    pub show_template_confirm_dialog: bool,

    // Delete table confirmation
    pub show_delete_table_dialog: bool,
    pub pending_delete_table: Option<String>,
}

/// Target for rename operation
#[derive(Debug, Clone, PartialEq)]
pub enum RenameTarget {
    Table(String),
    Column(String, String),
}

impl SchemaDesignerState {
    pub fn new() -> Self {
        Self {
            selected_schema: "public".to_string(),
            new_column_type: "text".to_string(),
            new_column_nullable: true,
            sql_preview_expanded: true,
            ..Default::default()
        }
    }

    /// Refresh the pending SQL based on schema changes
    pub fn refresh_pending_sql(&mut self) {
        self.pending_sql = generate_migration_sql(
            &self.original_schema,
            &self.working_schema,
            &self.selected_schema,
        );
    }

    /// Check if there are pending changes
    pub fn has_changes(&self) -> bool {
        !self.pending_sql.is_empty()
    }

    /// Push an action to the history and regenerate SQL
    pub fn push_action(&mut self, action: SchemaAction) {
        // Truncate any redo history
        self.action_history.truncate(self.action_index);

        // Push new action with timestamp
        self.action_history.push(TimestampedAction::new(action));
        self.action_index = self.action_history.len();

        // Limit history size
        if self.action_history.len() > 100 {
            self.action_history.remove(0);
            self.action_index -= 1;
        }

        // Refresh SQL preview
        self.refresh_pending_sql();
    }

    /// Undo the last action
    pub fn undo(&mut self) -> bool {
        if self.action_index == 0 {
            return false;
        }

        self.action_index -= 1;
        let action = self.action_history[self.action_index].action.clone();
        self.apply_inverse_action(&action);
        self.refresh_pending_sql();
        true
    }

    /// Redo the last undone action
    pub fn redo(&mut self) -> bool {
        if self.action_index >= self.action_history.len() {
            return false;
        }

        let action = self.action_history[self.action_index].action.clone();
        self.apply_action(&action);
        self.action_index += 1;
        self.refresh_pending_sql();
        true
    }

    /// Jump to a specific point in the history
    pub fn jump_to_action(&mut self, target_index: usize) {
        // Clamp target to valid range
        let target = target_index.min(self.action_history.len());

        // If target is before current position: undo until we reach target
        while self.action_index > target {
            self.action_index -= 1;
            let action = self.action_history[self.action_index].action.clone();
            self.apply_inverse_action(&action);
        }

        // If target is after current position: redo until we reach target
        while self.action_index < target {
            let action = self.action_history[self.action_index].action.clone();
            self.apply_action(&action);
            self.action_index += 1;
        }

        self.refresh_pending_sql();
    }

    /// Apply an action to the working schema
    fn apply_action(&mut self, action: &SchemaAction) {
        match action {
            SchemaAction::CreateTable(table) => {
                self.working_schema.tables.insert(table.name.clone(), table.clone());
            }
            SchemaAction::DropTable(table) => {
                self.working_schema.tables.remove(&table.name);
            }
            SchemaAction::RenameTable { old_name, new_name } => {
                if let Some(mut table) = self.working_schema.tables.remove(old_name) {
                    table.name = new_name.clone();
                    self.working_schema.tables.insert(new_name.clone(), table);
                }
            }
            SchemaAction::MoveTable { table, new_pos, .. } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.position = *new_pos;
                }
            }
            SchemaAction::AddColumn { table, column } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.columns.push(column.clone());
                }
            }
            SchemaAction::DropColumn { table, column } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.columns.retain(|c| c.name != column.name);
                }
            }
            SchemaAction::AlterColumn { table, new, .. } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    if let Some(c) = t.columns.iter_mut().find(|c| c.name == new.name) {
                        *c = new.clone();
                    }
                }
            }
            SchemaAction::RenameColumn { table, old_name, new_name } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    if let Some(c) = t.columns.iter_mut().find(|c| c.name == *old_name) {
                        c.name = new_name.clone();
                    }
                }
            }
            SchemaAction::CreateIndex { table, index } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.indexes.push(index.clone());
                }
            }
            SchemaAction::DropIndex { table, index } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.indexes.retain(|i| i.name != index.name);
                }
            }
            SchemaAction::AddConstraint { table, constraint } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.constraints.push(constraint.clone());
                }
            }
            SchemaAction::DropConstraint { table, constraint } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.constraints.retain(|c| c.name != constraint.name);
                }
            }
            SchemaAction::Batch(actions) => {
                for a in actions {
                    self.apply_action(a);
                }
            }
        }
    }

    /// Apply the inverse of an action (for undo)
    fn apply_inverse_action(&mut self, action: &SchemaAction) {
        match action {
            SchemaAction::CreateTable(table) => {
                self.working_schema.tables.remove(&table.name);
            }
            SchemaAction::DropTable(table) => {
                self.working_schema.tables.insert(table.name.clone(), table.clone());
            }
            SchemaAction::RenameTable { old_name, new_name } => {
                if let Some(mut table) = self.working_schema.tables.remove(new_name) {
                    table.name = old_name.clone();
                    self.working_schema.tables.insert(old_name.clone(), table);
                }
            }
            SchemaAction::MoveTable { table, old_pos, .. } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.position = *old_pos;
                }
            }
            SchemaAction::AddColumn { table, column } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.columns.retain(|c| c.name != column.name);
                }
            }
            SchemaAction::DropColumn { table, column } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.columns.push(column.clone());
                }
            }
            SchemaAction::AlterColumn { table, old, .. } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    if let Some(c) = t.columns.iter_mut().find(|c| c.name == old.name) {
                        *c = old.clone();
                    }
                }
            }
            SchemaAction::RenameColumn { table, old_name, new_name } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    if let Some(c) = t.columns.iter_mut().find(|c| c.name == *new_name) {
                        c.name = old_name.clone();
                    }
                }
            }
            SchemaAction::CreateIndex { table, index } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.indexes.retain(|i| i.name != index.name);
                }
            }
            SchemaAction::DropIndex { table, index } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.indexes.push(index.clone());
                }
            }
            SchemaAction::AddConstraint { table, constraint } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.constraints.retain(|c| c.name != constraint.name);
                }
            }
            SchemaAction::DropConstraint { table, constraint } => {
                if let Some(t) = self.working_schema.tables.get_mut(table) {
                    t.constraints.push(constraint.clone());
                }
            }
            SchemaAction::Batch(actions) => {
                // Apply inverse in reverse order
                for a in actions.iter().rev() {
                    self.apply_inverse_action(a);
                }
            }
        }
    }

    /// Reset the designer state
    pub fn reset(&mut self) {
        self.selected_database = None;
        self.original_schema = SchemaModel::default();
        self.working_schema = SchemaModel::default();
        self.selected_table = None;
        self.selected_column = None;
        self.selected_index = None;
        self.pending_sql.clear();
        self.action_history.clear();
        self.action_index = 0;
        self.canvas = CanvasState::default();
        self.visual_column_order.clear();
    }

    /// Reset for reload after migration - preserves action history
    pub fn reset_for_reload(&mut self) {
        self.original_schema = SchemaModel::default();
        self.working_schema = SchemaModel::default();
        self.selected_table = None;
        self.selected_column = None;
        self.selected_index = None;
        self.pending_sql.clear();
        // Preserve action_history and action_index
        self.canvas = CanvasState::default();
        self.visual_column_order.clear();
    }

    /// Reset form fields
    pub fn reset_new_table_form(&mut self) {
        self.new_table_name.clear();
        self.focus_new_table_name = true;
    }

    pub fn reset_new_column_form(&mut self) {
        self.new_column_name.clear();
        self.focus_new_column_name = true;
        self.new_column_type = "text".to_string();
        self.new_column_size.clear();
        self.new_column_nullable = true;
        self.new_column_default.clear();
        self.new_column_pk = false;
    }

    pub fn reset_new_index_form(&mut self) {
        self.new_index_name.clear();
        self.new_index_columns.clear();
        self.new_index_unique = false;
    }

    pub fn reset_new_fk_form(&mut self) {
        self.new_fk_column.clear();
        self.new_fk_ref_table.clear();
        self.new_fk_ref_column.clear();
        self.new_fk_on_delete = FkAction::NoAction;
    }

    /// Get the visual column order for a table (returns default order if not customized)
    pub fn get_visual_column_order(&self, table_name: &str) -> Vec<usize> {
        if let Some(order) = self.visual_column_order.get(table_name) {
            order.clone()
        } else if let Some(table) = self.working_schema.tables.get(table_name) {
            // Use semantic default order: PK first, FK second, timestamps last
            Self::compute_default_column_order(table)
        } else {
            Vec::new()
        }
    }

    /// Ensure visual column order is initialized for a table
    pub fn ensure_visual_column_order(&mut self, table_name: &str) {
        if !self.visual_column_order.contains_key(table_name) {
            if let Some(table) = self.working_schema.tables.get(table_name) {
                let order = Self::compute_default_column_order(table);
                self.visual_column_order.insert(table_name.to_string(), order);
            }
        }
    }

    /// Compute the default column order for a table:
    /// 1. Primary key columns first
    /// 2. Foreign key columns second
    /// 3. Regular columns in the middle
    /// 4. Timestamp columns last (ordered by name)
    fn compute_default_column_order(table: &TableModel) -> Vec<usize> {
        // Collect FK column names for lookup
        let fk_columns: HashSet<String> = table
            .constraints
            .iter()
            .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
            .flat_map(|c| c.columns.iter().cloned())
            .collect();

        // Categorize columns by index
        let mut pk_indices: Vec<usize> = Vec::new();
        let mut fk_indices: Vec<usize> = Vec::new();
        let mut regular_indices: Vec<usize> = Vec::new();
        let mut timestamp_indices: Vec<(usize, &str)> = Vec::new();

        for (idx, column) in table.columns.iter().enumerate() {
            if column.is_primary_key {
                pk_indices.push(idx);
            } else if fk_columns.contains(&column.name) {
                fk_indices.push(idx);
            } else if Self::is_timestamp_column(column) {
                timestamp_indices.push((idx, &column.name));
            } else {
                regular_indices.push(idx);
            }
        }

        // Sort timestamp columns by name
        timestamp_indices.sort_by(|a, b| a.1.cmp(b.1));

        // Build the final order: PK, FK, regular, timestamps
        let mut order = Vec::with_capacity(table.columns.len());
        order.extend(pk_indices);
        order.extend(fk_indices);
        order.extend(regular_indices);
        order.extend(timestamp_indices.into_iter().map(|(idx, _)| idx));

        order
    }

    /// Check if a column is a timestamp-type column
    fn is_timestamp_column(column: &ColumnModel) -> bool {
        let data_type = column.data_type.to_lowercase();
        data_type.contains("timestamp")
            || data_type == "date"
            || data_type == "time"
            || data_type.contains("timetz")
    }

    /// Reorder a column visually (from_idx and to_idx are visual indices)
    pub fn reorder_column_visually(&mut self, table_name: &str, from_visual_idx: usize, to_visual_idx: usize) {
        self.ensure_visual_column_order(table_name);

        if let Some(order) = self.visual_column_order.get_mut(table_name) {
            if from_visual_idx < order.len() && to_visual_idx < order.len() && from_visual_idx != to_visual_idx {
                let column_idx = order.remove(from_visual_idx);
                order.insert(to_visual_idx, column_idx);
            }
        }
    }

    /// Reset visual column order for a table (back to schema order)
    pub fn reset_visual_column_order(&mut self, table_name: &str) {
        self.visual_column_order.remove(table_name);
    }

    /// Reset all visual column orders
    pub fn reset_all_visual_column_orders(&mut self) {
        self.visual_column_order.clear();
    }

    /// Sync visual column order when columns are added/removed from schema
    pub fn sync_visual_column_order(&mut self, table_name: &str) {
        if let Some(table) = self.working_schema.tables.get(table_name) {
            let column_count = table.columns.len();

            if let Some(order) = self.visual_column_order.get_mut(table_name) {
                // Remove indices that are out of bounds
                order.retain(|&idx| idx < column_count);

                // Find new columns not yet in the order
                let new_indices: Vec<usize> = (0..column_count)
                    .filter(|i| !order.contains(i))
                    .collect();

                // Insert each new column at its appropriate position
                for new_idx in new_indices {
                    let column = &table.columns[new_idx];
                    let insert_pos = Self::find_insert_position_for_column(table, order, column);
                    order.insert(insert_pos, new_idx);
                }
            }
        }
    }

    /// Determine the insert position for a new column based on its type
    /// Order: PK columns first, FK columns second, regular columns, timestamps last
    fn find_insert_position_for_column(
        table: &TableModel,
        current_order: &[usize],
        new_column: &ColumnModel,
    ) -> usize {
        // Collect FK column names
        let fk_columns: HashSet<String> = table
            .constraints
            .iter()
            .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
            .flat_map(|c| c.columns.iter().cloned())
            .collect();

        // Determine category of the new column (0=PK, 1=FK, 2=regular, 3=timestamp)
        let new_category = if new_column.is_primary_key {
            0
        } else if fk_columns.contains(&new_column.name) {
            1
        } else if Self::is_timestamp_column(new_column) {
            3
        } else {
            2
        };

        // Find the appropriate insert position
        for (pos, &idx) in current_order.iter().enumerate() {
            if idx >= table.columns.len() {
                continue;
            }
            let col = &table.columns[idx];
            let col_category = if col.is_primary_key {
                0
            } else if fk_columns.contains(&col.name) {
                1
            } else if Self::is_timestamp_column(col) {
                3
            } else {
                2
            };

            // Insert before the first column of a higher category
            if col_category > new_category {
                return pos;
            }

            // For timestamps, insert alphabetically
            if new_category == 3 && col_category == 3 && new_column.name < col.name {
                return pos;
            }
        }

        // If no position found, append at the end
        current_order.len()
    }
}
