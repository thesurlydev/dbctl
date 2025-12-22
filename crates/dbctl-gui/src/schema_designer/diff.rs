use std::collections::HashSet;
use super::models::{ColumnModel, ConstraintModel, ConstraintType, IndexModel, SchemaModel, TableModel};

/// Compare two schemas and generate SQL statements
pub fn generate_migration_sql(
    original: &SchemaModel,
    modified: &SchemaModel,
    schema_name: &str,
) -> Vec<String> {
    let mut statements = Vec::new();

    // 1. Find dropped tables
    for (name, _table) in &original.tables {
        if !modified.tables.contains_key(name) {
            statements.push(format!(
                "DROP TABLE IF EXISTS \"{}\".\"{}\" CASCADE;",
                schema_name, name
            ));
        }
    }

    // 2. Find new tables
    for (name, table) in &modified.tables {
        if !original.tables.contains_key(name) {
            statements.push(generate_create_table(table, schema_name));
        }
    }

    // 3. Find altered tables
    for (name, modified_table) in &modified.tables {
        if let Some(original_table) = original.tables.get(name) {
            let alter_statements = diff_table(original_table, modified_table, schema_name);
            statements.extend(alter_statements);
        }
    }

    statements
}

fn diff_table(
    original: &TableModel,
    modified: &TableModel,
    schema_name: &str,
) -> Vec<String> {
    let mut statements = Vec::new();
    let table_name = &modified.name;

    // Find dropped columns
    let original_cols: HashSet<_> = original.columns.iter().map(|c| &c.name).collect();
    let modified_cols: HashSet<_> = modified.columns.iter().map(|c| &c.name).collect();

    for col in &original.columns {
        if !modified_cols.contains(&col.name) {
            statements.push(format!(
                "ALTER TABLE \"{}\".\"{}\" DROP COLUMN \"{}\";",
                schema_name, table_name, col.name
            ));
        }
    }

    // Find new columns
    for col in &modified.columns {
        if !original_cols.contains(&col.name) {
            statements.push(generate_add_column(table_name, col, schema_name));
        }
    }

    // Find altered columns
    for modified_col in &modified.columns {
        if let Some(original_col) = original.columns.iter().find(|c| c.name == modified_col.name) {
            let alter_stmts = diff_column(table_name, original_col, modified_col, schema_name);
            statements.extend(alter_stmts);
        }
    }

    // Diff indexes (skip primary key indexes)
    let original_indexes: HashSet<_> = original.indexes.iter()
        .filter(|i| !i.is_primary)
        .map(|i| &i.name)
        .collect();
    let modified_indexes: HashSet<_> = modified.indexes.iter()
        .filter(|i| !i.is_primary)
        .map(|i| &i.name)
        .collect();

    // Dropped indexes
    for idx in original.indexes.iter().filter(|i| !i.is_primary) {
        if !modified_indexes.contains(&idx.name) {
            statements.push(format!(
                "DROP INDEX IF EXISTS \"{}\".\"{}\";",
                schema_name, idx.name
            ));
        }
    }

    // New indexes
    for idx in modified.indexes.iter().filter(|i| !i.is_primary) {
        if !original_indexes.contains(&idx.name) {
            statements.push(generate_create_index(table_name, idx, schema_name));
        }
    }

    // Diff constraints (FKs only for now)
    let original_fks: HashSet<_> = original.constraints.iter()
        .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
        .map(|c| &c.name)
        .collect();
    let modified_fks: HashSet<_> = modified.constraints.iter()
        .filter(|c| c.constraint_type == ConstraintType::ForeignKey)
        .map(|c| &c.name)
        .collect();

    // Dropped FKs
    for constraint in original.constraints.iter().filter(|c| c.constraint_type == ConstraintType::ForeignKey) {
        if !modified_fks.contains(&constraint.name) {
            statements.push(format!(
                "ALTER TABLE \"{}\".\"{}\" DROP CONSTRAINT \"{}\";",
                schema_name, table_name, constraint.name
            ));
        }
    }

    // New FKs
    for constraint in modified.constraints.iter().filter(|c| c.constraint_type == ConstraintType::ForeignKey) {
        if !original_fks.contains(&constraint.name) {
            statements.push(generate_add_fk(table_name, constraint, schema_name));
        }
    }

    statements
}

fn diff_column(
    table_name: &str,
    original: &ColumnModel,
    modified: &ColumnModel,
    schema_name: &str,
) -> Vec<String> {
    let mut statements = Vec::new();

    // Check type change
    if original.data_type != modified.data_type {
        statements.push(format!(
            "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN \"{}\" TYPE {} USING \"{}\"::{};",
            schema_name, table_name, modified.name, modified.data_type, modified.name, modified.data_type
        ));
    }

    // Check nullability change
    if original.is_nullable != modified.is_nullable {
        if modified.is_nullable {
            statements.push(format!(
                "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN \"{}\" DROP NOT NULL;",
                schema_name, table_name, modified.name
            ));
        } else {
            statements.push(format!(
                "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN \"{}\" SET NOT NULL;",
                schema_name, table_name, modified.name
            ));
        }
    }

    // Check default value change
    if original.default_value != modified.default_value {
        if let Some(ref default) = modified.default_value {
            statements.push(format!(
                "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN \"{}\" SET DEFAULT {};",
                schema_name, table_name, modified.name, default
            ));
        } else {
            statements.push(format!(
                "ALTER TABLE \"{}\".\"{}\" ALTER COLUMN \"{}\" DROP DEFAULT;",
                schema_name, table_name, modified.name
            ));
        }
    }

    statements
}

fn generate_create_table(table: &TableModel, schema_name: &str) -> String {
    let mut sql = format!("CREATE TABLE \"{}\".\"{}\" (\n", schema_name, table.name);

    let mut definitions = Vec::new();

    // Column definitions
    for col in &table.columns {
        let mut def = format!("    \"{}\" {}", col.name, col.data_type);
        if !col.is_nullable {
            def.push_str(" NOT NULL");
        }
        if let Some(ref default) = col.default_value {
            def.push_str(&format!(" DEFAULT {}", default));
        }
        definitions.push(def);
    }

    // Primary key constraint
    let pk_cols: Vec<_> = table.columns.iter()
        .filter(|c| c.is_primary_key)
        .map(|c| format!("\"{}\"", c.name))
        .collect();
    if !pk_cols.is_empty() {
        definitions.push(format!("    PRIMARY KEY ({})", pk_cols.join(", ")));
    }

    sql.push_str(&definitions.join(",\n"));
    sql.push_str("\n);");
    sql
}

fn generate_add_column(table_name: &str, col: &ColumnModel, schema_name: &str) -> String {
    let mut sql = format!(
        "ALTER TABLE \"{}\".\"{}\" ADD COLUMN \"{}\" {}",
        schema_name, table_name, col.name, col.data_type
    );
    if !col.is_nullable {
        sql.push_str(" NOT NULL");
    }
    if let Some(ref default) = col.default_value {
        sql.push_str(&format!(" DEFAULT {}", default));
    }
    sql.push(';');
    sql
}

fn generate_create_index(table_name: &str, index: &IndexModel, schema_name: &str) -> String {
    let unique = if index.is_unique { "UNIQUE " } else { "" };
    let columns: Vec<_> = index.columns.iter().map(|c| format!("\"{}\"", c)).collect();
    format!(
        "CREATE {}INDEX \"{}\" ON \"{}\".\"{}\" USING {} ({});",
        unique, index.name, schema_name, table_name, index.index_type, columns.join(", ")
    )
}

fn generate_add_fk(table_name: &str, constraint: &ConstraintModel, schema_name: &str) -> String {
    let columns: Vec<_> = constraint.columns.iter().map(|c| format!("\"{}\"", c)).collect();
    let ref_table = constraint.foreign_table.as_ref().map(|s| s.as_str()).unwrap_or("");
    let ref_columns: Vec<_> = constraint.foreign_columns.as_ref()
        .map(|cols| cols.iter().map(|c| format!("\"{}\"", c)).collect())
        .unwrap_or_default();

    let mut sql = format!(
        "ALTER TABLE \"{}\".\"{}\" ADD CONSTRAINT \"{}\" FOREIGN KEY ({}) REFERENCES \"{}\".\"{}\" ({})",
        schema_name, table_name, constraint.name,
        columns.join(", "),
        schema_name, ref_table,
        ref_columns.join(", ")
    );

    if let Some(ref on_delete) = constraint.on_delete {
        sql.push_str(&format!(" ON DELETE {}", on_delete));
    }
    if let Some(ref on_update) = constraint.on_update {
        sql.push_str(&format!(" ON UPDATE {}", on_update));
    }

    sql.push(';');
    sql
}
