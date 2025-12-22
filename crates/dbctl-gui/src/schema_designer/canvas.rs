use std::collections::HashMap;

use eframe::egui;

use super::models::{ConstraintType, SchemaModel, TableModel};

/// Canvas state for rendering and interaction
#[derive(Clone)]
pub struct CanvasState {
    // Viewport
    pub offset: egui::Vec2,
    pub zoom: f32,

    // Interaction state
    pub dragging_table: Option<String>,
    pub drag_start_pos: Option<egui::Pos2>,
    pub drag_table_start: Option<egui::Pos2>,

    // Column dragging (visual reorder only)
    pub dragging_column: Option<(String, String)>, // (table_name, column_name)
    pub drag_column_start_idx: Option<usize>,
    pub drag_column_current_y: Option<f32>, // Screen Y position during drag

    // Panning
    pub panning: bool,
    pub pan_start: Option<egui::Pos2>,

    // Visual settings
    pub show_grid: bool,
    pub snap_to_grid: bool,
    pub grid_size: f32,

    // Hover state
    pub hovered_table: Option<String>,
    pub hovered_column: Option<(String, String)>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
            dragging_table: None,
            drag_start_pos: None,
            drag_table_start: None,
            dragging_column: None,
            drag_column_start_idx: None,
            drag_column_current_y: None,
            panning: false,
            pan_start: None,
            show_grid: true,
            snap_to_grid: true,
            grid_size: 20.0,
            hovered_table: None,
            hovered_column: None,
        }
    }
}

// Visual constants
const MIN_TABLE_WIDTH: f32 = 150.0;
const HEADER_HEIGHT: f32 = 28.0;
const ROW_HEIGHT: f32 = 22.0;
const TABLE_PADDING: f32 = 8.0;
const COLUMN_ICON_WIDTH: f32 = 24.0;
const CHAR_WIDTH: f32 = 6.6; // Approximate width per character for monospace 11pt font

// Colors
const COLOR_TABLE_HEADER: egui::Color32 = egui::Color32::from_rgb(66, 133, 244);
const COLOR_TABLE_HEADER_SELECTED: egui::Color32 = egui::Color32::from_rgb(25, 103, 210);
const COLOR_TABLE_BG: egui::Color32 = egui::Color32::from_rgb(255, 255, 255);
const COLOR_TABLE_BG_HOVER: egui::Color32 = egui::Color32::from_rgb(248, 249, 250);
const COLOR_TABLE_BG_SELECTED: egui::Color32 = egui::Color32::from_rgb(232, 240, 254);
const COLOR_TABLE_BORDER: egui::Color32 = egui::Color32::from_rgb(218, 220, 224);
const COLOR_TABLE_BORDER_SELECTED: egui::Color32 = egui::Color32::from_rgb(66, 133, 244);
const COLOR_FK_LINE: egui::Color32 = egui::Color32::from_rgb(102, 102, 200);
const COLOR_PK: egui::Color32 = egui::Color32::from_rgb(255, 193, 7);
const COLOR_FK: egui::Color32 = egui::Color32::from_rgb(156, 39, 176);

fn color_grid() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(200, 200, 200, 50)
}

/// Render the schema canvas
pub fn render_canvas(
    ui: &mut egui::Ui,
    schema: &SchemaModel,
    canvas: &mut CanvasState,
    selected_table: &Option<String>,
    selected_column: &Option<(String, String)>,
    visual_column_order: &HashMap<String, Vec<usize>>,
) -> CanvasResponse {
    let mut response = CanvasResponse::default();

    let canvas_rect = ui.available_rect_before_wrap();
    let canvas_response = ui.allocate_rect(canvas_rect, egui::Sense::click_and_drag());

    // Handle zoom with scroll
    if canvas_response.hovered() {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            let zoom_factor = 1.0 + scroll * 0.001;
            canvas.zoom = (canvas.zoom * zoom_factor).clamp(0.3, 3.0);
        }
    }

    // Handle panning with middle mouse or space+drag
    let space_held = ui.input(|i| i.key_down(egui::Key::Space));
    if canvas_response.drag_started_by(egui::PointerButton::Middle) ||
       (space_held && canvas_response.drag_started_by(egui::PointerButton::Primary)) {
        canvas.panning = true;
        canvas.pan_start = Some(canvas_response.interact_pointer_pos().unwrap_or_default());
    }

    if canvas.panning {
        if let Some(pos) = canvas_response.interact_pointer_pos() {
            if let Some(start) = canvas.pan_start {
                let delta = pos - start;
                canvas.offset += delta;
                canvas.pan_start = Some(pos);
            }
        }

        if canvas_response.drag_stopped() {
            canvas.panning = false;
            canvas.pan_start = None;
        }
    }

    let painter = ui.painter_at(canvas_rect);

    // Draw grid if enabled
    if canvas.show_grid {
        draw_grid(&painter, canvas_rect, canvas);
    }

    // Transform function: schema coords -> screen coords
    let transform = |pos: egui::Pos2| -> egui::Pos2 {
        egui::pos2(
            canvas_rect.left() + (pos.x + canvas.offset.x) * canvas.zoom,
            canvas_rect.top() + (pos.y + canvas.offset.y) * canvas.zoom,
        )
    };

    // Inverse transform: screen coords -> schema coords
    let _inverse_transform = |pos: egui::Pos2| -> egui::Pos2 {
        egui::pos2(
            (pos.x - canvas_rect.left()) / canvas.zoom - canvas.offset.x,
            (pos.y - canvas_rect.top()) / canvas.zoom - canvas.offset.y,
        )
    };

    // Draw FK lines first (behind tables)
    draw_fk_lines(&painter, schema, canvas, &transform, visual_column_order);

    // Reset hover state
    canvas.hovered_table = None;
    canvas.hovered_column = None;

    // Track hovered column visual index for drag operations
    let mut hovered_column_visual_idx: Option<usize> = None;

    // Draw tables and handle interactions
    let mut tables: Vec<_> = schema.tables.values().collect();
    tables.sort_by(|a, b| a.name.cmp(&b.name));

    for table in &tables {
        // Get visual column order for this table
        let default_order: Vec<usize> = (0..table.columns.len()).collect();
        let column_order = visual_column_order.get(&table.name).unwrap_or(&default_order);

        let table_rect = get_table_rect(table, canvas, &transform);

        // Check if hovered
        if let Some(pointer_pos) = canvas_response.interact_pointer_pos() {
            if table_rect.contains(pointer_pos) && !canvas.panning {
                canvas.hovered_table = Some(table.name.clone());

                // Check column hover (using visual order)
                let header_height = HEADER_HEIGHT * canvas.zoom;
                let row_height = ROW_HEIGHT * canvas.zoom;
                let y_offset = pointer_pos.y - table_rect.top() - header_height;
                if y_offset > 0.0 && !table.collapsed {
                    let visual_idx = (y_offset / row_height) as usize;
                    if visual_idx < column_order.len() {
                        let actual_idx = column_order[visual_idx];
                        if actual_idx < table.columns.len() {
                            canvas.hovered_column = Some((table.name.clone(), table.columns[actual_idx].name.clone()));
                            hovered_column_visual_idx = Some(visual_idx);
                        }
                    }
                }
            }
        }

        let is_selected = selected_table.as_ref() == Some(&table.name);
        let is_hovered = canvas.hovered_table.as_ref() == Some(&table.name);

        draw_table(
            &painter,
            table,
            canvas,
            &transform,
            is_selected,
            is_hovered,
            selected_column,
            column_order,
        );
    }

    // Handle table and column dragging
    if !canvas.panning {
        if canvas_response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(hovered_table) = &canvas.hovered_table {
                if let Some(hovered_col) = &canvas.hovered_column {
                    // Start column drag
                    if let Some(visual_idx) = hovered_column_visual_idx {
                        canvas.dragging_column = Some(hovered_col.clone());
                        canvas.drag_column_start_idx = Some(visual_idx);
                        canvas.drag_column_current_y = canvas_response.interact_pointer_pos().map(|p| p.y);
                    }
                } else {
                    // Start table drag (only if not on a column)
                    canvas.dragging_table = Some(hovered_table.clone());
                    canvas.drag_start_pos = canvas_response.interact_pointer_pos();
                    if let Some(table) = schema.tables.get(hovered_table) {
                        canvas.drag_table_start = Some(table.position);
                    }
                }
            }
        }

        // Handle column drag in progress
        if canvas.dragging_column.is_some() {
            if let Some(pos) = canvas_response.interact_pointer_pos() {
                canvas.drag_column_current_y = Some(pos.y);
            }
        }

        // Handle table drag in progress
        if canvas.dragging_table.is_some() {
            if let (Some(current_pos), Some(start_pos), Some(table_start)) = (
                canvas_response.interact_pointer_pos(),
                canvas.drag_start_pos,
                canvas.drag_table_start,
            ) {
                let delta = (current_pos - start_pos) / canvas.zoom;
                let mut new_pos = table_start + delta;

                // Snap to grid
                if canvas.snap_to_grid {
                    new_pos.x = (new_pos.x / canvas.grid_size).round() * canvas.grid_size;
                    new_pos.y = (new_pos.y / canvas.grid_size).round() * canvas.grid_size;
                }

                response.table_moved = Some((
                    canvas.dragging_table.clone().unwrap(),
                    table_start,
                    new_pos,
                ));
            }
        }

        if canvas_response.drag_stopped() {
            // Handle column drag end
            if let Some((table_name, _col_name)) = canvas.dragging_column.take() {
                if let Some(from_idx) = canvas.drag_column_start_idx.take() {
                    if let Some(current_y) = canvas.drag_column_current_y.take() {
                        // Calculate target visual index based on drop position
                        if let Some(table) = schema.tables.get(&table_name) {
                            let default_order: Vec<usize> = (0..table.columns.len()).collect();
                            let column_order = visual_column_order.get(&table_name).unwrap_or(&default_order);
                            let table_rect = get_table_rect(table, canvas, &transform);
                            let header_height = HEADER_HEIGHT * canvas.zoom;
                            let row_height = ROW_HEIGHT * canvas.zoom;

                            let y_offset = current_y - table_rect.top() - header_height;
                            let to_idx = if y_offset < 0.0 {
                                0
                            } else {
                                let idx = (y_offset / row_height) as usize;
                                idx.min(column_order.len().saturating_sub(1))
                            };

                            if from_idx != to_idx {
                                response.column_reordered = Some((table_name, from_idx, to_idx));
                            }
                        }
                    }
                }
            }

            // Handle table drag end
            if let Some(table_name) = canvas.dragging_table.take() {
                if let (Some(start), Some(table_start)) = (canvas.drag_start_pos, canvas.drag_table_start) {
                    if let Some(current) = canvas_response.interact_pointer_pos() {
                        let delta = (current - start) / canvas.zoom;
                        let mut new_pos = table_start + delta;

                        if canvas.snap_to_grid {
                            new_pos.x = (new_pos.x / canvas.grid_size).round() * canvas.grid_size;
                            new_pos.y = (new_pos.y / canvas.grid_size).round() * canvas.grid_size;
                        }

                        // Only record move if position actually changed
                        if (new_pos - table_start).length() > 1.0 {
                            response.table_move_completed = Some((table_name, table_start, new_pos));
                        }
                    }
                }
            }
            canvas.drag_start_pos = None;
            canvas.drag_table_start = None;
            canvas.drag_column_current_y = None;
        }

        // Handle click to select (only if not dragging)
        if canvas_response.clicked() && canvas.dragging_table.is_none() && canvas.dragging_column.is_none() {
            if let Some(hovered) = &canvas.hovered_table {
                if let Some(col) = &canvas.hovered_column {
                    response.column_clicked = Some(col.clone());
                } else {
                    response.table_clicked = Some(hovered.clone());
                }
            } else {
                // Clicked on empty space - deselect
                response.deselect = true;
            }
        }

        // Handle double-click to expand/collapse or edit
        if canvas_response.double_clicked() {
            if let Some(hovered) = &canvas.hovered_table {
                if canvas.hovered_column.is_some() {
                    response.column_double_clicked = canvas.hovered_column.clone();
                } else {
                    response.table_double_clicked = Some(hovered.clone());
                }
            }
        }
    }

    // Draw zoom indicator
    let zoom_text = format!("{:.0}%", canvas.zoom * 100.0);
    painter.text(
        egui::pos2(canvas_rect.right() - 50.0, canvas_rect.bottom() - 20.0),
        egui::Align2::RIGHT_BOTTOM,
        zoom_text,
        egui::FontId::proportional(12.0),
        egui::Color32::GRAY,
    );

    response
}

/// Response from canvas interaction
#[derive(Default)]
pub struct CanvasResponse {
    pub table_clicked: Option<String>,
    pub table_double_clicked: Option<String>,
    pub column_clicked: Option<(String, String)>,
    pub column_double_clicked: Option<(String, String)>,
    pub table_moved: Option<(String, egui::Pos2, egui::Pos2)>,
    pub table_move_completed: Option<(String, egui::Pos2, egui::Pos2)>,
    pub column_reordered: Option<(String, usize, usize)>, // (table_name, from_visual_idx, to_visual_idx)
    pub deselect: bool,
}

fn draw_grid(painter: &egui::Painter, rect: egui::Rect, canvas: &CanvasState) {
    let grid_size = canvas.grid_size * canvas.zoom;
    let offset_x = canvas.offset.x * canvas.zoom % grid_size;
    let offset_y = canvas.offset.y * canvas.zoom % grid_size;
    let grid_color = color_grid();

    let mut x = rect.left() + offset_x;
    while x < rect.right() {
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.0, grid_color),
        );
        x += grid_size;
    }

    let mut y = rect.top() + offset_y;
    while y < rect.bottom() {
        painter.line_segment(
            [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
            egui::Stroke::new(1.0, grid_color),
        );
        y += grid_size;
    }
}

/// Calculate the width needed for a table based on its columns
fn calculate_table_width(table: &TableModel) -> f32 {
    // Find the longest column text
    let max_col_len = table
        .columns
        .iter()
        .map(|col| {
            let nullable_marker = if col.is_nullable { "" } else { "*" };
            format!("{}{}: {}", col.name, nullable_marker, col.data_type).len()
        })
        .max()
        .unwrap_or(0);

    // Calculate width: padding + icon + text + padding
    let text_width = max_col_len as f32 * CHAR_WIDTH;
    let total_width = TABLE_PADDING + COLUMN_ICON_WIDTH + text_width + TABLE_PADDING;

    // Also consider table name width (centered in header)
    let header_width = table.name.len() as f32 * 8.0 + TABLE_PADDING * 4.0; // Header uses larger font

    total_width.max(header_width).max(MIN_TABLE_WIDTH)
}

fn get_table_rect(
    table: &TableModel,
    canvas: &CanvasState,
    transform: &impl Fn(egui::Pos2) -> egui::Pos2,
) -> egui::Rect {
    let width = calculate_table_width(table) * canvas.zoom;
    let height = if table.collapsed {
        HEADER_HEIGHT * canvas.zoom
    } else {
        (HEADER_HEIGHT + table.columns.len() as f32 * ROW_HEIGHT) * canvas.zoom
    };

    egui::Rect::from_min_size(transform(table.position), egui::vec2(width, height))
}

// Color for drag indicator
const COLOR_DRAG_INDICATOR: egui::Color32 = egui::Color32::from_rgb(66, 133, 244);

fn color_dragged_row_bg() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(66, 133, 244, 40)
}

fn draw_table(
    painter: &egui::Painter,
    table: &TableModel,
    canvas: &CanvasState,
    transform: &impl Fn(egui::Pos2) -> egui::Pos2,
    is_selected: bool,
    is_hovered: bool,
    selected_column: &Option<(String, String)>,
    column_order: &[usize],
) {
    let rect = get_table_rect(table, canvas, transform);
    let header_height = HEADER_HEIGHT * canvas.zoom;
    let row_height = ROW_HEIGHT * canvas.zoom;

    // Draw shadow
    painter.rect_filled(
        rect.translate(egui::vec2(3.0, 3.0)),
        4.0,
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 30),
    );

    // Draw body
    let bg_color = if is_selected {
        COLOR_TABLE_BG_SELECTED
    } else if is_hovered {
        COLOR_TABLE_BG_HOVER
    } else {
        COLOR_TABLE_BG
    };
    painter.rect_filled(rect, 4.0, bg_color);

    // Draw border
    let border_color = if is_selected {
        COLOR_TABLE_BORDER_SELECTED
    } else {
        COLOR_TABLE_BORDER
    };
    painter.rect_stroke(rect, 4.0, egui::Stroke::new(if is_selected { 2.0 } else { 1.0 }, border_color), egui::StrokeKind::Outside);

    // Draw header
    let header_rect = egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), header_height));
    let header_color = if is_selected { COLOR_TABLE_HEADER_SELECTED } else { COLOR_TABLE_HEADER };
    painter.rect_filled(
        header_rect,
        egui::CornerRadius { nw: 4, ne: 4, sw: 0, se: 0 },
        header_color,
    );

    // Draw table name
    painter.text(
        header_rect.center(),
        egui::Align2::CENTER_CENTER,
        &table.name,
        egui::FontId::proportional(13.0 * canvas.zoom),
        egui::Color32::WHITE,
    );

    // Draw collapse indicator
    let collapse_indicator = if table.collapsed { "+" } else { "-" };
    painter.text(
        egui::pos2(header_rect.right() - 12.0 * canvas.zoom, header_rect.center().y),
        egui::Align2::CENTER_CENTER,
        collapse_indicator,
        egui::FontId::proportional(12.0 * canvas.zoom),
        egui::Color32::WHITE,
    );

    // Check if we're dragging a column in this table
    let dragging_in_this_table = canvas.dragging_column
        .as_ref()
        .map(|(t, _)| t == &table.name)
        .unwrap_or(false);

    // Calculate drop target index if dragging
    let drop_target_idx = if dragging_in_this_table {
        if let Some(current_y) = canvas.drag_column_current_y {
            let y_offset = current_y - header_rect.bottom();
            if y_offset < 0.0 {
                Some(0)
            } else {
                let idx = (y_offset / row_height) as usize;
                Some(idx.min(column_order.len()))
            }
        } else {
            None
        }
    } else {
        None
    };

    // Draw columns if not collapsed (in visual order)
    if !table.collapsed {
        for (visual_idx, &actual_idx) in column_order.iter().enumerate() {
            if actual_idx >= table.columns.len() {
                continue;
            }
            let column = &table.columns[actual_idx];

            // Check if this column is being dragged
            let is_being_dragged = canvas.dragging_column
                .as_ref()
                .map(|(t, c)| t == &table.name && c == &column.name)
                .unwrap_or(false);

            let y = header_rect.bottom() + visual_idx as f32 * row_height;
            let col_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), y),
                egui::vec2(rect.width(), row_height),
            );

            // Background for dragged row
            if is_being_dragged {
                painter.rect_filled(col_rect, 0.0, color_dragged_row_bg());
            } else {
                // Highlight selected column
                let is_col_selected = selected_column
                    .as_ref()
                    .map(|(t, c)| t == &table.name && c == &column.name)
                    .unwrap_or(false);

                if is_col_selected {
                    painter.rect_filled(col_rect, 0.0, egui::Color32::from_rgb(200, 220, 255));
                }
            }

            // Draw drop indicator line
            if let Some(target_idx) = drop_target_idx {
                if visual_idx == target_idx {
                    // Draw line at top of this row
                    let line_y = col_rect.top();
                    painter.line_segment(
                        [egui::pos2(rect.left() + 4.0, line_y), egui::pos2(rect.right() - 4.0, line_y)],
                        egui::Stroke::new(2.0, COLOR_DRAG_INDICATOR),
                    );
                }
            }

            // Column icon (PK/FK)
            let icon_x = col_rect.left() + TABLE_PADDING * canvas.zoom;
            let icon_y = col_rect.center().y;

            if column.is_primary_key {
                painter.text(
                    egui::pos2(icon_x, icon_y),
                    egui::Align2::LEFT_CENTER,
                    "PK",
                    egui::FontId::monospace(9.0 * canvas.zoom),
                    COLOR_PK,
                );
            } else {
                // Check if this column is a FK
                let is_fk = table.constraints.iter().any(|c| {
                    c.constraint_type == ConstraintType::ForeignKey && c.columns.contains(&column.name)
                });
                if is_fk {
                    painter.text(
                        egui::pos2(icon_x, icon_y),
                        egui::Align2::LEFT_CENTER,
                        "FK",
                        egui::FontId::monospace(9.0 * canvas.zoom),
                        COLOR_FK,
                    );
                }
            }

            // Column name and type (slightly faded if being dragged)
            let text_x = col_rect.left() + (TABLE_PADDING + COLUMN_ICON_WIDTH) * canvas.zoom;
            let nullable_marker = if column.is_nullable { "" } else { "*" };
            let text = format!("{}{}: {}", column.name, nullable_marker, column.data_type);
            let text_color = if is_being_dragged {
                egui::Color32::from_rgb(100, 100, 100)
            } else {
                egui::Color32::BLACK
            };
            painter.text(
                egui::pos2(text_x, icon_y),
                egui::Align2::LEFT_CENTER,
                text,
                egui::FontId::monospace(11.0 * canvas.zoom),
                text_color,
            );
        }

        // Draw drop indicator at the end if dropping at the last position
        if let Some(target_idx) = drop_target_idx {
            if target_idx >= column_order.len() {
                let line_y = header_rect.bottom() + column_order.len() as f32 * row_height;
                painter.line_segment(
                    [egui::pos2(rect.left() + 4.0, line_y), egui::pos2(rect.right() - 4.0, line_y)],
                    egui::Stroke::new(2.0, COLOR_DRAG_INDICATOR),
                );
            }
        }
    }
}

fn draw_fk_lines(
    painter: &egui::Painter,
    schema: &SchemaModel,
    canvas: &CanvasState,
    transform: &impl Fn(egui::Pos2) -> egui::Pos2,
    visual_column_order: &HashMap<String, Vec<usize>>,
) {
    for table in schema.tables.values() {
        for constraint in &table.constraints {
            if constraint.constraint_type == ConstraintType::ForeignKey {
                if let (Some(ref_table_name), Some(ref_columns)) = (
                    &constraint.foreign_table,
                    &constraint.foreign_columns,
                ) {
                    if let Some(ref_table) = schema.tables.get(ref_table_name) {
                        // Find schema column indices
                        let from_schema_idx = table
                            .columns
                            .iter()
                            .position(|c| constraint.columns.contains(&c.name));
                        let to_schema_idx = ref_table
                            .columns
                            .iter()
                            .position(|c| ref_columns.contains(&c.name));

                        if let (Some(from_idx), Some(to_idx)) = (from_schema_idx, to_schema_idx) {
                            // Convert schema indices to visual indices
                            let from_visual_idx = schema_idx_to_visual_idx(&table.name, from_idx, visual_column_order, table.columns.len());
                            let to_visual_idx = schema_idx_to_visual_idx(ref_table_name, to_idx, visual_column_order, ref_table.columns.len());

                            // Determine optimal connection sides based on table positions
                            let from_table_center_x = transform(table.position).x + calculate_table_width(table) * canvas.zoom / 2.0;
                            let to_table_center_x = transform(ref_table.position).x + calculate_table_width(ref_table) * canvas.zoom / 2.0;

                            // Connect from the side closest to the other table
                            let (from_right_side, to_right_side) = if from_table_center_x < to_table_center_x {
                                // Source table is to the left of target: source right -> target left
                                (true, false)
                            } else {
                                // Source table is to the right of target: source left -> target right
                                (false, true)
                            };

                            let from_pos = get_column_connection_point(table, from_visual_idx, canvas, transform, from_right_side);
                            let to_pos = get_column_connection_point(ref_table, to_visual_idx, canvas, transform, to_right_side);

                            draw_fk_bezier(painter, from_pos, to_pos, COLOR_FK_LINE, from_right_side);
                        }
                    }
                }
            }
        }
    }
}

/// Convert a schema column index to a visual column index
fn schema_idx_to_visual_idx(
    table_name: &str,
    schema_idx: usize,
    visual_column_order: &HashMap<String, Vec<usize>>,
    column_count: usize,
) -> usize {
    if let Some(order) = visual_column_order.get(table_name) {
        // Find the visual position of this schema index
        order.iter().position(|&idx| idx == schema_idx).unwrap_or(schema_idx)
    } else {
        // No custom order, visual index equals schema index
        schema_idx.min(column_count.saturating_sub(1))
    }
}

fn get_column_connection_point(
    table: &TableModel,
    col_idx: usize,
    canvas: &CanvasState,
    transform: &impl Fn(egui::Pos2) -> egui::Pos2,
    right_side: bool,
) -> egui::Pos2 {
    let table_pos = transform(table.position);
    let header_height = HEADER_HEIGHT * canvas.zoom;
    let row_height = ROW_HEIGHT * canvas.zoom;
    let width = calculate_table_width(table) * canvas.zoom;

    let y = table_pos.y + header_height + (col_idx as f32 + 0.5) * row_height;
    let x = if right_side {
        table_pos.x + width
    } else {
        table_pos.x
    };

    egui::pos2(x, y)
}

fn draw_fk_bezier(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    color: egui::Color32,
    from_right_side: bool,
) {
    // Calculate control points for smooth bezier curve
    // Direction of curve depends on which side we're connecting from
    let dx = (to.x - from.x).abs().max(50.0) * 0.4;
    let (ctrl1, ctrl2) = if from_right_side {
        // Curve goes right from source, left into target
        (egui::pos2(from.x + dx, from.y), egui::pos2(to.x - dx, to.y))
    } else {
        // Curve goes left from source, right into target
        (egui::pos2(from.x - dx, from.y), egui::pos2(to.x + dx, to.y))
    };

    // Draw bezier curve using line segments
    let points = bezier_points(from, ctrl1, ctrl2, to, 24);
    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], egui::Stroke::new(2.0, color));
    }

    // Draw arrow at end
    draw_arrow_head(painter, points[points.len() - 2], to, color);

    // Draw circle at start
    painter.circle_filled(from, 4.0, color);
}

fn bezier_points(
    p0: egui::Pos2,
    p1: egui::Pos2,
    p2: egui::Pos2,
    p3: egui::Pos2,
    segments: usize,
) -> Vec<egui::Pos2> {
    let mut points = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;

        let x = mt3 * p0.x + 3.0 * mt2 * t * p1.x + 3.0 * mt * t2 * p2.x + t3 * p3.x;
        let y = mt3 * p0.y + 3.0 * mt2 * t * p1.y + 3.0 * mt * t2 * p2.y + t3 * p3.y;
        points.push(egui::pos2(x, y));
    }
    points
}

fn draw_arrow_head(
    painter: &egui::Painter,
    from: egui::Pos2,
    to: egui::Pos2,
    color: egui::Color32,
) {
    let dir = (to - from).normalized();
    let perp = egui::vec2(-dir.y, dir.x);

    let arrow_size = 8.0;
    let p1 = to - dir * arrow_size + perp * (arrow_size * 0.5);
    let p2 = to - dir * arrow_size - perp * (arrow_size * 0.5);

    painter.add(egui::Shape::convex_polygon(
        vec![to, p1, p2],
        color,
        egui::Stroke::NONE,
    ));
}

/// Auto-layout tables in a grid pattern
pub fn auto_layout_tables(schema: &mut SchemaModel) {
    let mut tables: Vec<_> = schema.tables.values_mut().collect();
    tables.sort_by(|a, b| a.name.cmp(&b.name));

    let cols = 3;
    let gap = 40.0; // Gap between tables
    let spacing_y = 300.0;
    let start_x = 50.0;
    let start_y = 50.0;

    // Calculate max width for each column
    let mut col_widths = vec![0.0_f32; cols];
    for (i, table) in tables.iter().enumerate() {
        let col = i % cols;
        col_widths[col] = col_widths[col].max(calculate_table_width(table));
    }

    for (i, table) in tables.iter_mut().enumerate() {
        let col = i % cols;
        let row = i / cols;

        // Calculate x position based on sum of previous column widths
        let x = start_x + (0..col).map(|c| col_widths[c] + gap).sum::<f32>();

        table.position = egui::pos2(x, start_y + row as f32 * spacing_y);
    }
}
