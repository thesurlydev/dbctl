use eframe::egui;

/// SQL keywords for syntax highlighting
const SQL_KEYWORDS: &[&str] = &[
    // DML
    "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "INTO", "VALUES", "SET",
    // Joins
    "JOIN", "INNER", "LEFT", "RIGHT", "OUTER", "FULL", "CROSS", "ON", "USING",
    // Clauses
    "ORDER", "BY", "GROUP", "HAVING", "LIMIT", "OFFSET", "DISTINCT", "AS", "UNION",
    "INTERSECT", "EXCEPT", "ALL", "ANY", "SOME",
    // Conditions
    "AND", "OR", "NOT", "IN", "BETWEEN", "LIKE", "ILIKE", "IS", "NULL", "EXISTS",
    // DDL
    "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "SCHEMA", "DATABASE",
    "CONSTRAINT", "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE", "CHECK",
    "DEFAULT", "CASCADE", "RESTRICT",
    // Other
    "EXPLAIN", "ANALYZE", "WITH", "RECURSIVE", "CASE", "WHEN", "THEN", "ELSE", "END",
    "CAST", "COALESCE", "NULLIF", "RETURNING", "CONFLICT", "DO", "NOTHING",
    // Transactions
    "BEGIN", "COMMIT", "ROLLBACK", "SAVEPOINT", "TRANSACTION",
    // Aggregates
    "COUNT", "SUM", "AVG", "MIN", "MAX", "ARRAY_AGG", "STRING_AGG",
];

/// SQL types for syntax highlighting
const SQL_TYPES: &[&str] = &[
    // Numeric
    "INTEGER", "INT", "SMALLINT", "BIGINT", "SERIAL", "BIGSERIAL",
    "NUMERIC", "DECIMAL", "REAL", "DOUBLE", "PRECISION", "FLOAT",
    // Text
    "TEXT", "VARCHAR", "CHAR", "CHARACTER", "VARYING",
    // Boolean
    "BOOLEAN", "BOOL", "TRUE", "FALSE",
    // Date/Time
    "DATE", "TIME", "TIMESTAMP", "TIMESTAMPTZ", "INTERVAL",
    // Binary
    "BYTEA", "BLOB",
    // Other
    "UUID", "JSON", "JSONB", "ARRAY", "XML",
];

/// Highlight SQL syntax and return a LayoutJob
pub fn highlight_sql(text: &str, font_id: egui::FontId) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();

    let keyword_color = egui::Color32::from_rgb(86, 156, 214);   // Blue
    let type_color = egui::Color32::from_rgb(78, 201, 176);      // Teal
    let string_color = egui::Color32::from_rgb(206, 145, 120);   // Orange
    let number_color = egui::Color32::from_rgb(181, 206, 168);   // Light green
    let comment_color = egui::Color32::from_rgb(106, 153, 85);   // Green
    let default_color = egui::Color32::from_rgb(212, 212, 212);  // Light gray

    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {

        // Comments
        if c == '-' && chars.peek() == Some(&'-') {
            // Single-line comment
            let mut comment = String::from(c);
            while let Some(&next) = chars.peek() {
                if next == '\n' {
                    break;
                }
                comment.push(chars.next().unwrap());
            }
            job.append(&comment, 0.0, egui::TextFormat {
                font_id: font_id.clone(),
                color: comment_color,
                ..Default::default()
            });
            continue;
        }

        // Strings (single quotes)
        if c == '\'' {
            let mut string_lit = String::from(c);
            while let Some(next) = chars.next() {
                string_lit.push(next);
                if next == '\'' {
                    // Check for escaped quote
                    if chars.peek() == Some(&'\'') {
                        string_lit.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
            }
            job.append(&string_lit, 0.0, egui::TextFormat {
                font_id: font_id.clone(),
                color: string_color,
                ..Default::default()
            });
            continue;
        }

        // Numbers
        if c.is_ascii_digit() || (c == '.' && chars.peek().is_some_and(|n| n.is_ascii_digit())) {
            let mut number = String::from(c);
            while let Some(&next) = chars.peek() {
                if next.is_ascii_digit() || next == '.' {
                    number.push(chars.next().unwrap());
                } else {
                    break;
                }
            }
            job.append(&number, 0.0, egui::TextFormat {
                font_id: font_id.clone(),
                color: number_color,
                ..Default::default()
            });
            continue;
        }

        // Identifiers/keywords
        if c.is_alphabetic() || c == '_' {
            let mut word = String::from(c);
            while let Some(&next) = chars.peek() {
                if next.is_alphanumeric() || next == '_' {
                    word.push(chars.next().unwrap());
                } else {
                    break;
                }
            }

            let upper = word.to_uppercase();
            let color = if SQL_KEYWORDS.contains(&upper.as_str()) {
                keyword_color
            } else if SQL_TYPES.contains(&upper.as_str()) {
                type_color
            } else {
                default_color
            };

            job.append(&word, 0.0, egui::TextFormat {
                font_id: font_id.clone(),
                color,
                ..Default::default()
            });
            continue;
        }

        // Default: operators, whitespace, punctuation
        job.append(&c.to_string(), 0.0, egui::TextFormat {
            font_id: font_id.clone(),
            color: default_color,
            ..Default::default()
        });
    }

    job
}
