pub const SYSTEM_CONFIG_PATH: &str = "/usr/share/fin/config.toml";
pub const SYSTEM_CSS_DIR: &str = "/usr/share/fin";

use crate::error::ConfigError;
use im::{HashMap, Vector};
use serde::Deserialize;
use std::env;
use std::path::{Path, PathBuf};

/// The configuration for a set of action buttons, as stored in the config file.
pub type ButtonConfigs = Vector<ButtonConfig>;

/// Returns the default number of columns.
fn default_columns() -> usize {
    1
}

/// Custom deserializer that converts a standard `Vec` into an immutable `Vector`.
fn deserialize_vector<'de, D, T>(deserializer: D) -> Result<Vector<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Clone,
{
    let vec = Vec::<T>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

/// Returns the default value for the `use_gtk_theme` flag.
fn default_use_gtk_theme() -> bool {
    false
}

/// Returns the default window width ratio.
fn default_window_width_ratio() -> f64 {
    DEFAULT_WINDOW_WIDTH_RATIO
}

/// Returns the default window height ratio.
fn default_window_height_ratio() -> f64 {
    DEFAULT_WINDOW_HEIGHT_RATIO
}

/// Returns the default button font ratio.
fn default_button_font_ratio() -> f64 {
    DEFAULT_BUTTON_FONT_RATIO
}

/// Represents commands and button configuration for a specific desktop environment.
#[derive(Deserialize, Debug, Clone)]
pub struct DECommands {
    #[serde(default = "default_columns")]
    pub columns: usize,
    pub buttons: ButtonConfigs,
}

/// Represents layout configuration parameters for the application window.
#[derive(Deserialize, Debug, Clone)]
pub struct LayoutConfig {
    #[serde(default = "default_window_width_ratio")]
    pub(crate) window_width_ratio: f64,
    #[serde(default = "default_window_height_ratio")]
    pub(crate) window_height_ratio: f64,
    #[serde(default = "default_button_font_ratio")]
    pub(crate) button_font_ratio: f64,
}

/// Main configuration structure loaded from the TOML configuration file.
#[derive(Deserialize, Debug, Clone, Default)]
pub struct Config {
    /// The title of the application window.
    pub(crate) title: String,
    #[serde(default = "default_columns")]
    pub(crate) columns: usize,
    /// A list of button configurations.
    #[serde(default, deserialize_with = "deserialize_vector")]
    pub(crate) buttons: ButtonConfigs,
    /// Flag indicating whether to use the system GTK theme.
    #[serde(default = "default_use_gtk_theme")]
    pub use_gtk_theme: bool,
    /// Optional user-specified stylesheet path.
    #[serde(default, alias = "stylesheet")]
    pub css_path: Option<String>,
    /// Default commands mapped by desktop environment.
    #[serde(default)]
    pub(crate) default_commands: HashMap<String, DECommands>,
    /// Optional layout configuration.
    #[serde(default)]
    pub(crate) layout: Option<LayoutConfig>,
    /// The name of the theme to load (e.g., "default").
    #[serde(default)]
    pub(crate) theme: Option<String>,
}

impl Config {
    /// Returns a copy of this config with `columns` replaced (avoids struct-update at call sites).
    #[must_use]
    pub fn with_columns(self, columns: usize) -> Self {
        Self { columns, ..self }
    }
}

/// Represents the configuration for an individual button.
#[derive(Deserialize, Debug, Clone)]
pub struct ButtonConfig {
    pub label: String,
    pub command: String,
    /// Optional list of extra CSS classes for styling.
    #[serde(default)]
    pub css_classes: Option<Vec<String>>,
    /// Optional widget name (ID) for unique styling.
    #[serde(default)]
    pub widget_name: Option<String>,
}

// ---------------------------------------------------------------------
// Configuration loading (pure + IO split)
// ---------------------------------------------------------------------

/// Pure: parse a TOML string into a `Config`, with validation.
pub fn parse_config(content: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(content)?;
    if config.columns == 0 {
        return Err(ConfigError::Validation(
            "columns must be greater than 0".into(),
        ));
    }
    Ok(config)
}

/// IO: read a file and delegate to `parse_config`.
pub fn load_config(path: &Path) -> Result<Config, ConfigError> {
    let content = std::fs::read_to_string(path)?;
    parse_config(&content)
}

// ---------------------------------------------------------------------
// Config path resolution
// ---------------------------------------------------------------------

fn xdg_config_path() -> Option<PathBuf> {
    env::var_os("XDG_CONFIG_HOME").map(PathBuf::from)
}

fn home_config_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(".config/fin/config.toml"))
}

fn system_config_path() -> PathBuf {
    PathBuf::from(SYSTEM_CONFIG_PATH)
}

/// Resolve the configuration path, preferring XDG config, then `$HOME/.config`,
/// then the system path. The selected path must exist and be a file.
#[must_use]
pub fn determine_config_path() -> PathBuf {
    xdg_config_path()
        .or_else(home_config_path)
        .filter(|p| p.exists() && p.is_file())
        .unwrap_or_else(system_config_path)
}

/// Returns the desktop environment-specific commands and columns, falling back
/// to the default commands, then to the top-level config.
#[must_use]
pub fn get_commands_for_de<'a>(de: &str, config: &'a Config) -> (&'a ButtonConfigs, usize) {
    config
        .default_commands
        .get(de)
        .or_else(|| config.default_commands.get("default"))
        .map(|de_cmd| (&de_cmd.buttons, de_cmd.columns))
        .unwrap_or((&config.buttons, config.columns))
}

const DEFAULT_WINDOW_WIDTH_RATIO: f64 = 0.3;
const DEFAULT_WINDOW_HEIGHT_RATIO: f64 = 0.3;
const DEFAULT_BUTTON_FONT_RATIO: f64 = 0.14;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{get_temp_dir, init_env};
    use im::{hashmap, vector};
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn load_config_valid_file() {
        init_env();
        let tmp = get_temp_dir().unwrap();
        let config_path = tmp.path().join("valid_config.toml");
        fs::write(
            &config_path,
            r#"
        title = "Valid Config"
        columns = 1
        buttons = [
            { label = "Log out", command = "echo 'logout command'" }
        ]
    "#,
        )
        .unwrap();
        let result = load_config(&config_path);
        assert!(result.is_ok());
    }

    #[test]
    fn load_config_nonexistent_file() {
        init_env();
        let path = PathBuf::from("tests/fixtures/nonexistent_config.toml");
        let result = load_config(&path);
        assert!(result.is_err());
    }

    #[test]
    fn get_commands_for_de_with_default() {
        init_env();
        let config = Config {
            title: "Test".to_string(),
            columns: 1,
            buttons: vector![],
            use_gtk_theme: false,
            css_path: None,
            default_commands: hashmap! {
                "default".to_string() => DECommands {
                    columns: 2,
                    buttons: vector![
                        ButtonConfig {
                            label: "Default".to_string(),
                            command: "echo default".to_string(),
                            css_classes: None,
                            widget_name: None,
                        }
                    ]
                }
            },
            layout: None,
            theme: None,
        };
        let (commands, columns) = get_commands_for_de("unknown_de", &config);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label, "Default");
        assert_eq!(columns, 2);
    }

    #[test]
    fn get_commands_for_de_with_override() {
        init_env();
        let config = Config {
            title: "Test".to_string(),
            columns: 1,
            buttons: vector![],
            use_gtk_theme: false,
            css_path: None,
            default_commands: hashmap! {
                "test_de".to_string() => DECommands {
                    columns: 1,
                    buttons: vector![
                        ButtonConfig {
                            label: "Test".to_string(),
                            command: "echo test".to_string(),
                            css_classes: None,
                            widget_name: None,
                        }
                    ]
                }
            },
            layout: None,
            theme: None,
        };
        let (commands, columns) = get_commands_for_de("test_de", &config);
        assert_eq!(commands.len(), 1);
        assert_eq!(columns, 1);
    }
}
