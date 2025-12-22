use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

/// Theme mode selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeMode {
    Light,
    Dark,
    #[default]
    System,
}

impl ThemeMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::System => "System",
        }
    }

    pub fn cycle(&self) -> Self {
        match self {
            Self::Light => Self::Dark,
            Self::Dark => Self::System,
            Self::System => Self::Light,
        }
    }
}

/// Color palette for the application
#[derive(Debug, Clone)]
pub struct Theme {
    pub is_dark: bool,

    // Canvas colors (schema designer)
    pub table_header: Color32,
    pub table_header_selected: Color32,
    pub table_bg: Color32,
    pub table_bg_hover: Color32,
    pub table_bg_selected: Color32,
    pub table_border: Color32,
    pub table_border_selected: Color32,
    pub canvas_grid: Color32,
    pub table_shadow: Color32,

    // Schema designer specific
    pub fk_line: Color32,
    pub pk_badge: Color32,
    pub fk_badge: Color32,
    pub drag_indicator: Color32,
    pub dragged_row_bg: Color32,
    pub column_selected_bg: Color32,

    // Text colors
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_on_accent: Color32,
    pub text_muted: Color32,

    // Syntax highlighting
    pub syntax_keyword: Color32,
    pub syntax_type: Color32,
    pub syntax_string: Color32,
    pub syntax_number: Color32,
    pub syntax_comment: Color32,
    pub syntax_default: Color32,

    // UI status colors
    pub status_running: Color32,
    pub status_stopped: Color32,
    pub warning: Color32,
    pub success: Color32,

    // Command palette
    pub backdrop: Color32,
    pub selection_bg: Color32,
}

impl Theme {
    pub fn light() -> Self {
        Self {
            is_dark: false,

            // Canvas colors
            table_header: Color32::from_rgb(66, 133, 244),
            table_header_selected: Color32::from_rgb(25, 103, 210),
            table_bg: Color32::from_rgb(255, 255, 255),
            table_bg_hover: Color32::from_rgb(248, 249, 250),
            table_bg_selected: Color32::from_rgb(232, 240, 254),
            table_border: Color32::from_rgb(218, 220, 224),
            table_border_selected: Color32::from_rgb(66, 133, 244),
            canvas_grid: Color32::from_rgba_unmultiplied(200, 200, 200, 50),
            table_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 30),

            // Schema designer
            fk_line: Color32::from_rgb(102, 102, 200),
            pk_badge: Color32::from_rgb(255, 193, 7),
            fk_badge: Color32::from_rgb(156, 39, 176),
            drag_indicator: Color32::from_rgb(66, 133, 244),
            dragged_row_bg: Color32::from_rgba_unmultiplied(66, 133, 244, 40),
            column_selected_bg: Color32::from_rgb(200, 220, 255),

            // Text
            text_primary: Color32::from_rgb(0, 0, 0),
            text_secondary: Color32::from_rgb(100, 100, 100),
            text_on_accent: Color32::WHITE,
            text_muted: Color32::GRAY,

            // Syntax (VS Code light-inspired)
            syntax_keyword: Color32::from_rgb(0, 0, 255),
            syntax_type: Color32::from_rgb(38, 127, 153),
            syntax_string: Color32::from_rgb(163, 21, 21),
            syntax_number: Color32::from_rgb(9, 134, 88),
            syntax_comment: Color32::from_rgb(0, 128, 0),
            syntax_default: Color32::from_rgb(0, 0, 0),

            // Status
            status_running: Color32::from_rgb(40, 167, 69),
            status_stopped: Color32::from_rgb(220, 53, 69),
            warning: Color32::from_rgb(220, 53, 69),
            success: Color32::from_rgb(40, 167, 69),

            // Command palette
            backdrop: Color32::from_rgba_unmultiplied(0, 0, 0, 128),
            selection_bg: Color32::from_rgb(60, 100, 180),
        }
    }

    pub fn dark() -> Self {
        Self {
            is_dark: true,

            // Canvas colors
            table_header: Color32::from_rgb(55, 90, 150),
            table_header_selected: Color32::from_rgb(70, 120, 200),
            table_bg: Color32::from_rgb(40, 44, 52),
            table_bg_hover: Color32::from_rgb(50, 54, 62),
            table_bg_selected: Color32::from_rgb(45, 55, 72),
            table_border: Color32::from_rgb(70, 74, 82),
            table_border_selected: Color32::from_rgb(100, 149, 237),
            canvas_grid: Color32::from_rgba_unmultiplied(100, 100, 100, 40),
            table_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 60),

            // Schema designer
            fk_line: Color32::from_rgb(140, 140, 220),
            pk_badge: Color32::from_rgb(255, 193, 7),
            fk_badge: Color32::from_rgb(186, 104, 200),
            drag_indicator: Color32::from_rgb(100, 149, 237),
            dragged_row_bg: Color32::from_rgba_unmultiplied(100, 149, 237, 40),
            column_selected_bg: Color32::from_rgb(55, 70, 100),

            // Text
            text_primary: Color32::from_rgb(220, 220, 220),
            text_secondary: Color32::from_rgb(160, 160, 160),
            text_on_accent: Color32::WHITE,
            text_muted: Color32::from_rgb(128, 128, 128),

            // Syntax (VS Code dark-inspired)
            syntax_keyword: Color32::from_rgb(86, 156, 214),
            syntax_type: Color32::from_rgb(78, 201, 176),
            syntax_string: Color32::from_rgb(206, 145, 120),
            syntax_number: Color32::from_rgb(181, 206, 168),
            syntax_comment: Color32::from_rgb(106, 153, 85),
            syntax_default: Color32::from_rgb(212, 212, 212),

            // Status
            status_running: Color32::from_rgb(75, 181, 67),
            status_stopped: Color32::from_rgb(244, 67, 54),
            warning: Color32::from_rgb(244, 67, 54),
            success: Color32::from_rgb(75, 181, 67),

            // Command palette
            backdrop: Color32::from_rgba_unmultiplied(0, 0, 0, 180),
            selection_bg: Color32::from_rgb(50, 80, 140),
        }
    }

    /// Get the appropriate theme based on mode
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Light => Self::light(),
            ThemeMode::Dark => Self::dark(),
            ThemeMode::System => {
                if detect_system_dark_mode() {
                    Self::dark()
                } else {
                    Self::light()
                }
            }
        }
    }
}

/// Detect if the system is using dark mode
pub fn detect_system_dark_mode() -> bool {
    matches!(dark_light::detect(), Ok(dark_light::Mode::Dark))
}

/// GUI configuration stored on disk
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuiConfig {
    pub theme_mode: ThemeMode,
}

impl GuiConfig {
    /// Load GUI config from disk
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save GUI config to disk
    pub fn save(&self) {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(config_path, content);
        }
    }

    fn config_path() -> std::path::PathBuf {
        dirs::home_dir()
            .unwrap_or_default()
            .join(".dbctl")
            .join("gui-config.json")
    }
}
