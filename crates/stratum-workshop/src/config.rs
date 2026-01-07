//! Workshop configuration and layout persistence
//!
//! Stores user preferences including panel visibility and layout ratios.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default file browser width ratio (proportion of total width)
const DEFAULT_FILE_BROWSER_RATIO: f32 = 0.2;
/// Default output panel height ratio (proportion of main area height)
const DEFAULT_OUTPUT_RATIO: f32 = 0.3;
/// Default data explorer width ratio (proportion of remaining width after file browser)
const DEFAULT_DATA_EXPLORER_RATIO: f32 = 0.2;

/// Configuration for panel visibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelVisibility {
    pub file_browser: bool,
    pub output: bool,
    pub repl: bool,
    #[serde(default = "default_data_explorer_visible")]
    pub data_explorer: bool,
}

fn default_data_explorer_visible() -> bool {
    false // Hidden by default, user can toggle on
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            file_browser: true,
            output: true,
            repl: true,
            data_explorer: false, // Hidden by default
        }
    }
}

/// Configuration for panel layout ratios
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Ratio of file browser width to total window width (0.0 - 1.0)
    pub file_browser_ratio: f32,
    /// Ratio of output panel height to main area height (0.0 - 1.0)
    pub output_ratio: f32,
    /// Ratio of data explorer width (right side) to remaining width (0.0 - 1.0)
    #[serde(default = "default_data_explorer_ratio")]
    pub data_explorer_ratio: f32,
    /// Panel visibility settings
    pub visibility: PanelVisibility,
}

fn default_data_explorer_ratio() -> f32 {
    DEFAULT_DATA_EXPLORER_RATIO
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            file_browser_ratio: DEFAULT_FILE_BROWSER_RATIO,
            output_ratio: DEFAULT_OUTPUT_RATIO,
            data_explorer_ratio: DEFAULT_DATA_EXPLORER_RATIO,
            visibility: PanelVisibility::default(),
        }
    }
}

impl LayoutConfig {
    /// Clamp ratios to valid range
    pub fn clamp_ratios(&mut self) {
        self.file_browser_ratio = self.file_browser_ratio.clamp(0.1, 0.5);
        self.output_ratio = self.output_ratio.clamp(0.1, 0.6);
        self.data_explorer_ratio = self.data_explorer_ratio.clamp(0.1, 0.4);
    }
}

/// Main workshop configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkshopConfig {
    /// Layout configuration
    pub layout: LayoutConfig,
    /// Recently opened files
    pub recent_files: Vec<PathBuf>,
    /// Recently opened folders
    pub recent_folders: Vec<PathBuf>,
    /// Last opened folder
    pub last_folder: Option<PathBuf>,
    /// Window size (width, height)
    pub window_size: (u32, u32),
    /// Window position (x, y) - None means centered
    pub window_position: Option<(i32, i32)>,
}

impl Default for WorkshopConfig {
    fn default() -> Self {
        Self {
            layout: LayoutConfig::default(),
            recent_files: Vec::new(),
            recent_folders: Vec::new(),
            last_folder: None,
            window_size: (1200, 800),
            window_position: None,
        }
    }
}

impl WorkshopConfig {
    /// Maximum number of recent files/folders to keep
    const MAX_RECENT: usize = 10;

    /// Get the config file path
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("stratum").join("workshop.toml"))
    }

    /// Load configuration from disk
    pub fn load() -> Self {
        Self::config_path()
            .and_then(|path| std::fs::read_to_string(&path).ok())
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    }

    /// Save configuration to disk
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_path().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "Config directory not found")
        })?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        std::fs::write(path, content)
    }

    /// Add a file to recent files list
    pub fn add_recent_file(&mut self, path: PathBuf) {
        // Remove if already exists to move to front
        self.recent_files.retain(|p| p != &path);
        self.recent_files.insert(0, path);
        self.recent_files.truncate(Self::MAX_RECENT);
    }

    /// Add a folder to recent folders list
    pub fn add_recent_folder(&mut self, path: PathBuf) {
        // Remove if already exists to move to front
        self.recent_folders.retain(|p| p != &path);
        self.recent_folders.insert(0, path.clone());
        self.recent_folders.truncate(Self::MAX_RECENT);
        self.last_folder = Some(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WorkshopConfig::default();
        assert!(config.layout.visibility.file_browser);
        assert!(config.layout.visibility.output);
        assert!(config.layout.visibility.repl);
        assert_eq!(config.window_size, (1200, 800));
    }

    #[test]
    fn test_clamp_ratios() {
        let mut layout = LayoutConfig {
            file_browser_ratio: 0.0,
            output_ratio: 1.0,
            data_explorer_ratio: 0.5,
            visibility: PanelVisibility::default(),
        };
        layout.clamp_ratios();
        assert_eq!(layout.file_browser_ratio, 0.1);
        assert_eq!(layout.output_ratio, 0.6);
        assert_eq!(layout.data_explorer_ratio, 0.4);
    }

    #[test]
    fn test_recent_files() {
        let mut config = WorkshopConfig::default();
        let path1 = PathBuf::from("/test/file1.strat");
        let path2 = PathBuf::from("/test/file2.strat");

        config.add_recent_file(path1.clone());
        config.add_recent_file(path2.clone());

        assert_eq!(config.recent_files.len(), 2);
        assert_eq!(config.recent_files[0], path2);
        assert_eq!(config.recent_files[1], path1);

        // Adding same file moves it to front
        config.add_recent_file(path1.clone());
        assert_eq!(config.recent_files.len(), 2);
        assert_eq!(config.recent_files[0], path1);
    }

    #[test]
    fn test_config_serialization() {
        let config = WorkshopConfig::default();
        let toml_str = toml::to_string_pretty(&config).expect("Failed to serialize");
        let parsed: WorkshopConfig = toml::from_str(&toml_str).expect("Failed to deserialize");
        assert_eq!(config.window_size, parsed.window_size);
    }
}
