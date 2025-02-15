/*!
This module implements the Finë application launcher.
It loads a configuration file, resolves CSS paths (user-specified or default),
loads a theme (user or system provided), and builds a GTK-based UI with action buttons.
*/

mod theming;

use anyhow::{anyhow, Context, Result};
use clap::{parser::ValueSource, Arg, Command};
use glib::Propagation;
use gtk4::gdk::Monitor;
use gtk4::{
    gdk, prelude::*, AlertDialog, Application, ApplicationWindow, Button, CssProvider,
    EventControllerFocus, EventControllerKey, Grid,
};
use im::{HashMap, Vector};
use log::{error, info, warn};
use serde::Deserialize;
use std::cell::Cell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::rc::Rc;

// ---------------------------------------------------------------------
// Constants for UI layout and system paths
// ---------------------------------------------------------------------
const GRID_COLUMN_SPACING: i32 = 10;
const GRID_ROW_SPACING: i32 = 10;
const GRID_MARGIN: i32 = 20;
const DEFAULT_WINDOW_WIDTH_RATIO: f64 = 0.3;
const DEFAULT_WINDOW_HEIGHT_RATIO: f64 = 0.3;
const DEFAULT_BUTTON_FONT_RATIO: f64 = 0.14;
const SYSTEM_CONFIG_PATH: &str = "/usr/share/fin/config.toml";
const SYSTEM_CSS_DIR: &str = "/usr/share/fin";
const SYSTEM_THEME_DIR: &str = "/usr/share/fin/themes";

// ---------------------------------------------------------------------
// Type Aliases
// ---------------------------------------------------------------------
type ButtonConfigs = Vector<ButtonConfig>; // Configuration type.
type Buttons = Vector<Button>; // Widget type.

// ---------------------------------------------------------------------
// 1. Configuration & CSS Resolution Helpers
// ---------------------------------------------------------------------

/// Determines the configuration file path by checking, in order:
/// 1. The path specified by the `XDG_CONFIG_HOME` environment variable.
/// 2. The path derived from the `HOME` environment variable (`$HOME/.config/fin/config.toml`).
/// 3. Falls back to the system-wide configuration file at `SYSTEM_CONFIG_PATH`.
///
/// # Returns
///
/// A `PathBuf` pointing to the configuration file.
fn determine_config_path() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME").map(|home| {
                PathBuf::from(home)
                    .join(".config")
                    .join("fin")
                    .join("config.toml")
            })
        })
        .filter(|path| path.exists() && path.is_file())
        .unwrap_or(PathBuf::from(SYSTEM_CONFIG_PATH))
}

// ---------------------------------------------------------------------
// CSS Resolution Helpers
// ---------------------------------------------------------------------

/// Resolves a relative CSS path based on the directory of `config_path`.
fn resolve_relative_path(user_css: &str, config_path: &Path) -> PathBuf {
    let user_path = PathBuf::from(user_css);
    if user_path.is_absolute() {
        user_path
    } else {
        config_path
            .parent()
            .map(|parent| parent.join(&user_path))
            .unwrap_or_else(|| user_path.clone())
    }
}

/// Resolves a CSS path using a clear match-based logic.
///
/// # Parameters
/// - `config_css`: Optional user CSS path from configuration.
/// - `config_path`: The configuration file path.
/// - `default_css`: The default CSS file path.
///
/// # Returns
/// A resolved `PathBuf`—either the user-provided one (if it exists) or the default.
fn resolve_css_path(config_css: Option<String>, config_path: &Path, default_css: &Path) -> PathBuf {
    match config_css {
        Some(user_css) => {
            let resolved = resolve_relative_path(&user_css, config_path);
            if resolved.exists() {
                resolved
            } else {
                warn!(
                    "User provided CSS '{}' not found. Falling back to default CSS '{}'.",
                    resolved.display(),
                    default_css.display()
                );
                default_css.to_path_buf()
            }
        }
        None => default_css.to_path_buf(),
    }
}

/// Loads the system-configured CSS file as a fallback.
///
/// # Parameters
/// - `default_css`: The default CSS file path used for fallback.
///
/// # Returns
/// An `Option<PathBuf>` with the system CSS file if found.
fn load_system_css(_default_css: &Path) -> Option<PathBuf> {
    let system_config_path =
        env::var("FIN_SYSTEM_CONFIG").unwrap_or_else(|_| SYSTEM_CONFIG_PATH.to_string());
    let system_config_content = fs::read_to_string(&system_config_path).ok()?;
    let system_config: SystemConfig = toml::from_str(&system_config_content).ok()?;
    let css_str = system_config.css_path?;
    let system_css_path = Path::new(SYSTEM_CSS_DIR).join(css_str);
    if system_css_path.exists() {
        info!(
            "Using system-configured CSS at '{}'.",
            system_css_path.display()
        );
        Some(system_css_path)
    } else {
        warn!(
            "System-configured CSS '{}' does not exist.",
            system_css_path.display()
        );
        None
    }
}

/// Resolves and selects the final CSS path according to the following rules:
/// 1. If `use_gtk_theme` is true, returns `None`.
/// 2. Otherwise, uses `resolve_css_path` and, if that file does not exist, checks
///    the system config file for an alternative.
///
/// # Parameters
///
/// - `config_css`: Optional user-specified CSS path.
/// - `config_path`: The path to the configuration file.
/// - `default_css`: The default CSS file path.
/// - `use_gtk_theme`: If true, uses the system GTK theme.
///
/// # Returns
///
/// A `Result` with an `Option<PathBuf>` for the CSS file.
fn select_css_path(
    config_css: Option<String>,
    config_path: &Path,
    default_css: &Path,
    use_gtk_theme: bool,
) -> Result<Option<PathBuf>> {
    if use_gtk_theme {
        info!("use_gtk_theme is true; using system GTK theme (no custom CSS).");
        return Ok(None);
    }
    let css_path = resolve_css_path(config_css, config_path, default_css);
    if css_path.exists() {
        info!("Using CSS file at '{}'.", css_path.display());
        Ok(Some(css_path))
    } else {
        info!("Default CSS not found. Checking system config.");
        Ok(load_system_css(default_css))
    }
}
/// Structure representing the system configuration read from a TOML file.
#[derive(Deserialize)]
struct SystemConfig {
    #[serde(default, alias = "stylesheet")]
    css_path: Option<String>,
}

/// Loads the theme configuration and returns a CSS string that defines CSS custom properties.
///
/// The function attempts to load a user theme (from `$HOME/.config/fin/themes/<theme_name>.toml`)
/// and falls back to the system theme (`/usr/share/fin/themes/<theme_name>.toml`) if needed.
/// On error, it returns a fallback CSS string with default values.
///
/// # Parameters
///
/// - `config`: Reference to the main configuration which may specify the theme name.
///
/// # Returns
///
/// A `String` containing the CSS rules for the theme.
fn get_theme_css(config: &Config) -> String {
    let theme_name = config.theme.as_deref().unwrap_or("default");
    let user_theme_path = dirs::config_dir()
        .map(|p| {
            p.join("fin")
                .join("themes")
                .join(format!("{}.toml", theme_name))
        })
        .filter(|p| p.exists());
    let system_theme_path = PathBuf::from(SYSTEM_THEME_DIR).join(format!("{}.toml", theme_name));
    let theme_path = user_theme_path.unwrap_or(system_theme_path);

    load_theme(&theme_path)
        .map(|theme| {
            format!(
                ":root {{
                   --background: {};
                   --foreground: {};
                   --cursor-color: {};
                   --cursor-text: {};
                   --selection-background: {};
                   --selection-foreground: {};
                   --palette-0: {};
                   --palette-1: {};
                   --palette-2: {};
                   --palette-3: {};
                   --palette-4: {};
                   --palette-5: {};
                   --palette-6: {};
                   --palette-7: {};
                   --palette-8: {};
                   --palette-9: {};
                   --palette-10: {};
                   --palette-11: {};
                   --palette-12: {};
                   --palette-13: {};
                   --palette-14: {};
                   --palette-15: {};
                }}",
                theme.background,
                theme.foreground,
                theme.cursor_color,
                theme.cursor_text,
                theme.selection_background,
                theme.selection_foreground,
                theme.palette0,
                theme.palette1,
                theme.palette2,
                theme.palette3,
                theme.palette4,
                theme.palette5,
                theme.palette6,
                theme.palette7,
                theme.palette8,
                theme.palette9,
                theme.palette10,
                theme.palette11,
                theme.palette12,
                theme.palette13,
                theme.palette14,
                theme.palette15,
            )
        })
        .unwrap_or_else(|e| {
            error!(
                "Error loading theme from path: {}: {:?}",
                theme_path.display(),
                e
            );
            // Fallback defaults:
            ":root {
                   --background: rgba(35, 33, 54, 1);
                   --foreground: rgba(224, 222, 244, 1);
                   --cursor-color: rgba(224, 222, 244, 1);
                   --cursor-text: rgba(35, 33, 54, 1);
                   --selection-background: rgba(68, 65, 90, 1);
                   --selection-foreground: rgba(224, 222, 244, 1);
                   --palette-0: rgba(57, 53, 82, 1);
                   --palette-1: rgba(235, 111, 146, 1);
                   --palette-2: rgba(62, 143, 176, 1);
                   --palette-3: rgba(246, 193, 119, 1);
                   --palette-4: rgba(156, 207, 216, 1);
                   --palette-5: rgba(196, 167, 231, 1);
                   --palette-6: rgba(234, 154, 151, 1);
                   --palette-7: rgba(224, 222, 244, 1);
                   --palette-8: rgba(110, 106, 134, 1);
                   --palette-9: rgba(235, 111, 146, 1);
                   --palette-10: rgba(62, 143, 176, 1);
                   --palette-11: rgba(246, 193, 119, 1);
                   --palette-12: rgba(156, 207, 216, 1);
                   --palette-13: rgba(196, 167, 231, 1);
                   --palette-14: rgba(234, 154, 151, 1);
                   --palette-15: rgba(224, 222, 244, 1);
                }"
            .to_string()
        })
}

// ---------------------------------------------------------------------
// Configuration Structures and Loading
// ---------------------------------------------------------------------

/// Returns the default number of columns.
fn default_columns() -> usize {
    1
}

/// Custom deserializer that converts a standard `Vec` into an immutable `Vector`.
fn deserialize_vector<'de, D, T>(deserializer: D) -> std::result::Result<Vector<T>, D::Error>
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

/// Represents commands and button configuration for a specific desktop environment.
#[derive(Deserialize, Debug, Clone)]
struct DECommands {
    #[serde(default = "default_columns")]
    columns: usize,
    buttons: ButtonConfigs,
}

/// Represents layout configuration parameters for the application window.
#[derive(Deserialize, Debug, Clone)]
struct LayoutConfig {
    #[serde(default = "default_window_width_ratio")]
    window_width_ratio: f64,
    #[serde(default = "default_window_height_ratio")]
    window_height_ratio: f64,
    #[serde(default = "default_button_font_ratio")]
    button_font_ratio: f64,
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

/// Main configuration structure loaded from the TOML configuration file.
#[derive(Deserialize, Debug, Clone)]
struct Config {
    /// The title of the application window.
    title: String,
    #[serde(default = "default_columns")]
    columns: usize,
    /// A list of button configurations.
    #[serde(default, deserialize_with = "deserialize_vector")]
    buttons: ButtonConfigs,
    /// Flag indicating whether to use the system GTK theme.
    #[serde(default = "default_use_gtk_theme")]
    use_gtk_theme: bool,
    /// Optional user-specified stylesheet path.
    #[serde(default, alias = "stylesheet")]
    css_path: Option<String>,
    /// Default commands mapped by desktop environment.
    #[serde(default)]
    default_commands: HashMap<String, DECommands>,
    /// Optional layout configuration.
    #[serde(default)]
    layout: Option<LayoutConfig>,
    /// The name of the theme to load (e.g., "default").
    #[serde(default)]
    theme: Option<String>,
}

/// Represents the configuration for an individual button.
#[derive(Deserialize, Debug, Clone)]
struct ButtonConfig {
    /// The label displayed on the button.
    label: String,
    /// The command executed when the button is clicked.
    command: String,
}

// ---------------------------------------------------------------------
// Configuration Loading
// ---------------------------------------------------------------------

/// Loads and parses the configuration file located at `path`.
///
/// # Parameters
/// - `path`: The path to the configuration file.
///
/// # Returns
/// A `Result` containing the parsed `Config` or an error with context.
fn load_config(path: &Path) -> Result<Config> {
    info!("Loading config from {:?}", path);
    let config_content = fs::read_to_string(path)
        .with_context(|| format!("Could not read config file at path: {:?}", path))?;
    info!("Config file content: {}", config_content);
    let config: Config = toml::from_str(&config_content)
        .context(format!("TOML deserialization error in file: {:?}", path))?;
    if config.columns == 0 {
        return Err(anyhow!(
            "Invalid configuration in file {:?}: columns must be greater than 0",
            path
        ));
    }
    Ok(config)
}

/// Returns the desktop environment-specific commands and columns.
/// Falls back to the default commands if no specific configuration is found.
///
/// # Parameters
/// - `de`: The detected desktop environment.
/// - `config`: The overall configuration.
///
/// # Returns
/// A tuple `(buttons, columns)`.
fn get_commands_for_de(de: &str, config: &Config) -> (ButtonConfigs, usize) {
    if let Some(de_cmd) = config.default_commands.get(de) {
        (de_cmd.buttons.clone(), de_cmd.columns)
    } else if let Some(default_cmd) = config.default_commands.get("default") {
        (default_cmd.buttons.clone(), default_cmd.columns)
    } else {
        (config.buttons.clone(), config.columns)
    }
}

// ---------------------------------------------------------------------
// UI and Navigation Functions
// ---------------------------------------------------------------------

/// Executes the given shell command.
/// This function abstracts the command execution logic from the button creation.
///
/// # Parameters
/// - `command`: The shell command to execute.
///
/// # Returns
/// A `Result` indicating success or failure.
fn execute_command(command: &str) -> Result<()> {
    if command.is_empty() {
        return Ok(());
    }
    ProcessCommand::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .with_context(|| format!("Failed to execute command '{}'", command))?;
    Ok(())
}

/// Creates an action button with the specified label and command.
/// The button is styled with the "action-button" CSS class and shows a tooltip with the command.
///
/// # Parameters
/// - `app`: Reference to the GTK application.
/// - `label`: The label for the button.
/// - `command`: The command to execute when the button is clicked.
///
/// # Returns
/// A `Button` widget.
fn create_action_button(app: &Application, label: &str, command: &str) -> Button {
    let button = Button::with_label(label);
    button.add_css_class("action-button");
    button.set_tooltip_text(Some(&format!("Executes command: {}", command)));
    let command_string = command.to_string();
    let app_clone = app.clone();
    button.connect_clicked(move |_| {
        if let Err(e) = execute_command(&command_string) {
            let err_msg = format!("Failed to execute command '{}': {}", command_string, e);
            error!("{}", err_msg);
            // Now showing an error dialog instead of just logging.
            show_error_dialog(&app_clone, &err_msg);
        }
        app_clone.quit();
    });
    button
}

/// Displays an error message using a GTK modal dialog.
///
/// # Parameters
/// - `app`: Reference to the GTK application.
/// - `message`: The error message to display.
fn show_error_dialog(_app: &Application, message: &str) {
    let dialog = AlertDialog::builder()
        .modal(true)
        .message("Error")
        .detail(message)
        .buttons(&["Ok"][..])
        .build();

    // Show the dialog without a parent.
    dialog.show(None::<&ApplicationWindow>);
}

/// Sets up the focus chain by assigning the first button as the focus child in the grid.
///
/// # Parameters
/// - `grid`: The grid widget.
/// - `buttons`: A vector of button widgets.
fn setup_focus_chain(grid: &Grid, buttons: &Buttons) {
    if let Some(first_button) = buttons.get(0) {
        grid.set_focus_child(Some(first_button));
    }
}

/// Sets up a focus controller for the window so that losing focus quits the application.
fn setup_focus_controller(window: &ApplicationWindow, app: &Application) {
    let app_clone = app.clone();
    let focus_controller = EventControllerFocus::new();
    focus_controller.connect_leave(move |_| {
        app_clone.quit();
    });
    window.add_controller(focus_controller);
}

/// Helper function to calculate the new index when moving up.
/// Wraps to the bottom in the same column if necessary.
fn index_up(current: usize, total: usize, columns: usize) -> usize {
    if let Some(new_index) = current.checked_sub(columns) {
        new_index.min(total.saturating_sub(1))
    } else {
        let col = current % columns;
        let num_rows = total.div_ceil(columns);
        let last_row = num_rows - 1;
        let candidate = last_row * columns + col;
        if candidate < total {
            candidate
        } else {
            total - 1
        }
    }
}

/// Helper function to calculate the new index when moving down.
/// Wraps to the top in the same column if necessary.
fn index_down(current: usize, total: usize, columns: usize) -> usize {
    let candidate = current + columns;
    if candidate < total {
        candidate
    } else {
        current % columns
    }
}

/// Helper function to calculate the new index when moving left.
fn index_left(current: usize, total: usize) -> usize {
    (current + total - 1) % total
}

/// Helper function to calculate the new index when moving right.
fn index_right(current: usize, total: usize) -> usize {
    (current + 1) % total
}

/// Computes a new index for button focus when an arrow key is pressed.
///
/// # Parameters
/// - `current`: The current focused button index.
/// - `total`: The total number of buttons.
/// - `columns`: The number of columns in the grid.
/// - `key`: The key that was pressed.
///
/// # Returns
/// The new index after applying the arrow key movement.
fn calculate_new_index_for_arrow(
    current: usize,
    total: usize,
    columns: usize,
    key: gdk::Key,
) -> usize {
    match key {
        gdk::Key::Up | gdk::Key::KP_Up => index_up(current, total, columns),
        gdk::Key::Down | gdk::Key::KP_Down => index_down(current, total, columns),
        gdk::Key::Left | gdk::Key::KP_Left => index_left(current, total),
        gdk::Key::Right | gdk::Key::KP_Right => index_right(current, total),
        _ => current,
    }
}

/// Computes a new index for button focus when the Tab key is pressed.
///
/// # Parameters
/// - `index`: The current index.
/// - `total`: The total number of buttons.
/// - `forward`: If true, moves forward; if false, moves backward.
///
/// # Returns
/// The new index after tabbing.
fn calculate_new_index_for_tab(index: usize, total: usize, forward: bool) -> usize {
    if forward {
        (index + 1) % total
    } else {
        (index + total - 1) % total
    }
}

/// Sets up key handlers for the window to handle navigation and button activation.
///
/// # Parameters
/// - `window`: The application window.
/// - `app`: The GTK application.
/// - `buttons`: A reference-counted vector of button widgets.
/// - `columns`: The number of columns in the grid.
fn setup_key_handlers(
    window: &ApplicationWindow,
    app: &Application,
    buttons: Rc<Buttons>,
    columns: usize,
) {
    window.set_can_focus(true);
    window.grab_focus();
    if let Some(first_button) = buttons.get(0) {
        first_button.grab_focus();
    }
    let current_index = Rc::new(Cell::new(0));
    let controller = EventControllerKey::new();

    let app_clone = app.clone();
    let buttons_ref = Rc::clone(&buttons);
    let current_index_clone = Rc::clone(&current_index);

    controller.connect_key_pressed(move |_, key_value, _hardware_keycode, _state| {
        let total = buttons_ref.len();
        let current = current_index_clone.get();
        let new_index = match key_value {
            gdk::Key::Escape => {
                app_clone.quit();
                return Propagation::Stop;
            }
            gdk::Key::Return => {
                if let Some(button) = buttons_ref.get(current) {
                    button.emit_clicked();
                }
                return Propagation::Stop;
            }
            gdk::Key::Tab => calculate_new_index_for_tab(current, total, true),
            gdk::Key::ISO_Left_Tab => calculate_new_index_for_tab(current, total, false),
            key => calculate_new_index_for_arrow(current, total, columns, key),
        };
        if new_index != current {
            current_index_clone.set(new_index);
            if let Some(button) = buttons_ref.get(new_index) {
                button.grab_focus();
            }
        }
        Propagation::Stop
    });

    window.add_controller(controller);
}

/// Composes a grid layout and attaches action buttons to it.
///
/// # Parameters
/// - `app`: The GTK application.
/// - `buttons`: A vector of button configurations.
/// - `columns`: The number of columns in the grid.
///
/// # Returns
/// A `Result` containing a tuple of the `Grid` widget and a vector of created `Button` widgets.
fn compose_grid(
    app: &Application,
    buttons: &ButtonConfigs,
    columns: usize,
) -> Result<(Grid, Buttons)> {
    let grid = create_grid();
    let all_buttons: Buttons = buttons
        .iter()
        .enumerate()
        .map(|(index, cfg)| {
            let button = create_action_button(app, &cfg.label, &cfg.command);
            let row = index / columns;
            let col = index % columns;
            grid.attach(&button, col as i32, row as i32, 1, 1);
            info!(
                "Attached button '{}' at row {}, col {}",
                cfg.label, row, col
            );
            button
        })
        .collect();
    Ok((grid, all_buttons))
}

/// Creates a new `Grid` widget with default spacing and margins.
///
/// # Returns
/// A `Grid` widget.
fn create_grid() -> Grid {
    Grid::builder()
        .column_homogeneous(true)
        .row_homogeneous(true)
        .column_spacing(GRID_COLUMN_SPACING)
        .row_spacing(GRID_ROW_SPACING)
        .margin_top(GRID_MARGIN)
        .margin_bottom(GRID_MARGIN)
        .margin_start(GRID_MARGIN)
        .margin_end(GRID_MARGIN)
        .build()
}

/// Detects the current desktop environment using the `XDG_CURRENT_DESKTOP` or `DESKTOP_SESSION` environment variables.
///
/// # Returns
/// A `String` representing the detected desktop environment (in lowercase).
fn detect_desktop_environment() -> String {
    env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| env::var("DESKTOP_SESSION"))
        .unwrap_or_default()
        .to_lowercase()
        .split(':')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

// ---------------------------------------------------------------------
// Theme Loading
// ---------------------------------------------------------------------

/// Represents a theme with a color palette and styling definitions.
#[derive(Deserialize, Debug, Clone)]
struct Theme {
    palette0: String,
    palette1: String,
    palette2: String,
    palette3: String,
    palette4: String,
    palette5: String,
    palette6: String,
    palette7: String,
    palette8: String,
    palette9: String,
    palette10: String,
    palette11: String,
    palette12: String,
    palette13: String,
    palette14: String,
    palette15: String,
    background: String,
    foreground: String,
    cursor_color: String,
    cursor_text: String,
    selection_background: String,
    selection_foreground: String,
}

/// Loads a theme from a TOML file located at `path`.
///
/// # Parameters
/// - `path`: The path to the theme file.
///
/// # Returns
/// A `Result` containing the loaded `Theme` or an error with context.
fn load_theme<P: AsRef<Path>>(path: P) -> Result<Theme> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Could not read theme file at path: {:?}", path.as_ref()))?;
    let theme: Theme = toml::from_str(&content)
        .with_context(|| format!("Theme deserialization error in file: {:?}", path.as_ref()))?;
    Ok(theme)
}

// ---------------------------------------------------------------------
// Building the UI
// ---------------------------------------------------------------------

/// Loads combined CSS into a single `CssProvider`.
///
/// # Parameters
/// - `dynamic_css`: The dynamically generated CSS (e.g., for button font sizes).
/// - `user_css`: The CSS loaded from a user-specified file.
/// - `theme_css`: The CSS generated from the theme.
///
/// # Returns
/// A `CssProvider` loaded with the combined CSS.
fn load_combined_css(dynamic_css: &str, user_css: &str, theme_css: &str) -> CssProvider {
    let combined_css = format!("{}\n{}\n{}", dynamic_css, user_css, theme_css);
    let provider = CssProvider::new();
    provider.load_from_string(&combined_css);
    provider
}

/// Builds the application UI by combining dynamic CSS, user CSS, and theme CSS.
/// It creates the main window, applies the combined CSS, and sets up the grid layout and key handlers.
///
/// # Parameters
/// - `app`: The GTK application.
/// - `config`: The application configuration.
/// - `stylesheet_path`: An optional path to the user-provided stylesheet.
/// - `buttons`: A vector of button configurations to display.
///
/// # Returns
/// A `Result` indicating success or failure.
fn build_ui(
    app: &Application,
    config: &Config,
    stylesheet_path: Option<PathBuf>,
    buttons: &ButtonConfigs,
) -> Result<()> {
    if config.columns == 0 {
        return Err(anyhow!(
            "Invalid configuration: columns must be greater than 0"
        ));
    }
    if buttons.is_empty() {
        return Err(anyhow!(
            "No buttons to display; please check your configuration"
        ));
    }

    let display =
        gdk::Display::default().ok_or_else(|| anyhow!("Could not get default display"))?;
    let primary_monitor = display
        .monitors()
        .item(0)
        .and_then(|obj| obj.downcast::<Monitor>().ok())
        .ok_or_else(|| anyhow!("Could not get primary monitor"))?;
    let geom = primary_monitor.geometry();

    let layout = config.layout.clone().unwrap_or(LayoutConfig {
        window_width_ratio: DEFAULT_WINDOW_WIDTH_RATIO,
        window_height_ratio: DEFAULT_WINDOW_HEIGHT_RATIO,
        button_font_ratio: DEFAULT_BUTTON_FONT_RATIO,
    });

    let window_width = (geom.width() as f64 * layout.window_width_ratio) as i32;
    let window_height = (geom.height() as f64 * layout.window_height_ratio) as i32;

    let window = ApplicationWindow::builder()
        .application(app)
        .title(&config.title)
        .default_width(window_width)
        .default_height(window_height)
        .build();
    window.set_decorated(false);
    window.set_transient_for(None::<&ApplicationWindow>);
    window.set_resizable(false);
    window.set_tooltip_text(Some("Finë logout manager window"));

    // Generate dynamic CSS based on window height.
    let dynamic_css = format!(
        ".action-button {{ font-size: {}px; }}",
        (window_height as f64 * layout.button_font_ratio) as i32
    );

    // Load user CSS from the provided stylesheet path unless using GTK theme.
    let user_css = if !config.use_gtk_theme {
        stylesheet_path
            .as_ref()
            .and_then(|path| fs::read_to_string(path).ok())
            .unwrap_or_default()
    } else {
        String::new()
    };

    let theme_css = get_theme_css(config);
    let provider = load_combined_css(&dynamic_css, &user_css, &theme_css);
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let (grid, all_buttons) = compose_grid(app, buttons, config.columns)?;
    let buttons_rc = Rc::new(all_buttons);

    setup_focus_chain(&grid, &buttons_rc);
    window.set_child(Some(&grid));
    setup_focus_controller(&window, app);
    setup_key_handlers(&window, app, Rc::clone(&buttons_rc), config.columns);
    window.present();

    Ok(())
}

// ---------------------------------------------------------------------
// Main Entry Point
// ---------------------------------------------------------------------

/// Main function that initializes logging, parses command-line arguments,
/// loads the configuration, and launches the GTK application.
fn main() -> Result<()> {
    // Initialize logging as early as possible.
    env_logger::init();

    let matches = Command::new("fin")
        .version("0.1.0")
        .about("Finë Application")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .num_args(1),
        )
        .get_matches();

    if matches.contains_id("config") {
        match matches.value_source("config") {
            Some(ValueSource::CommandLine) => info!("`config` set by user"),
            _ => info!("`config` is defaulted"),
        }
    }

    let config_path = matches
        .get_one::<String>("config")
        .map(PathBuf::from)
        .unwrap_or_else(determine_config_path);

    let config = load_config(&config_path)
        .with_context(|| format!("Failed to load configuration from {:?}", config_path))?;
    let de = detect_desktop_environment();
    let (commands, de_columns) = get_commands_for_de(&de, &config);
    let config = Config {
        columns: de_columns,
        ..config
    };

    let default_css = PathBuf::from(format!("{}/style.css", SYSTEM_CSS_DIR));

    let stylesheet_path = select_css_path(
        config.css_path.clone(),
        &config_path,
        &default_css,
        config.use_gtk_theme,
    )?;

    let app = Application::builder()
        .application_id("com.fin.launcher")
        .build();

    let config_clone = config.clone();
    app.connect_activate(move |app| {
        if let Err(e) = build_ui(app, &config_clone, stylesheet_path.clone(), &commands) {
            error!("Error building UI: {:?}", e);
            std::process::exit(1);
        }
    });

    app.run();
    Ok(())
}

// ---------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use im::{hashmap, vector};
    use log::info;
    use std::env;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex, Once};
    use std::time::{SystemTime, UNIX_EPOCH};
    use tempfile::TempDir;

    static INIT: Once = Once::new();
    /// Initializes the test environment and logger.
    fn init_env() {
        INIT.call_once(|| {
            let _ = env_logger::builder().is_test(true).try_init();
            env::set_var("FIN_SYSTEM_CONFIG", "/nonexistent/path/config.toml");
        });
    }

    /// Returns a dedicated temporary directory for tests using tempfile.
    fn get_temp_dir() -> Result<TempDir> {
        let tmp_dir = TempDir::new()?;
        Ok(tmp_dir)
    }

    #[test]
    fn load_config_valid_file() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.path().join("valid_config.toml");
        fs::write(
            &config_path,
            r#"
        title = "Valid Config"
        columns = 1
        buttons = [
            { label = "Log out", command = "echo 'logout command'" }
        ]
    "#,
        )?;
        let result = load_config(&config_path);
        assert!(result.is_ok());
        Ok(())
    }

    #[test]
    fn test_select_css_path_missing_user_css_fallback_to_default() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let unique_suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let config_path = tmp_dir
            .path()
            .join(format!("dummy_config_{}.toml", unique_suffix));
        fs::write(&config_path, "dummy config")?;
        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp_dir
            .path()
            .join(format!("default_{}.css", unique_suffix));
        fs::write(&default_css, "button { background-color: green; }")?;

        info!("Config path: {:?}", config_path);
        info!("Default CSS path: {:?}", default_css);

        let result = select_css_path(user_css, &config_path, &default_css, false)?;
        info!("Selected CSS path: {:?}", result);
        assert_eq!(result, Some(default_css.clone()));

        Ok(())
    }

    #[test]
    fn load_config_nonexistent_file() {
        init_env();
        let path = PathBuf::from("tests/fixtures/nonexistent_config.toml");
        let result = load_config(&path);
        assert!(result.is_err());
    }

    #[test]
    fn new_index_for_arrow_up() {
        init_env();
        let index = calculate_new_index_for_arrow(3, 6, 2, gdk::Key::Up);
        assert_eq!(index, index_up(3, 6, 2));
    }

    #[test]
    fn new_index_for_arrow_down() {
        init_env();
        let index = calculate_new_index_for_arrow(1, 6, 2, gdk::Key::Down);
        assert_eq!(index, index_down(1, 6, 2));
    }

    #[test]
    fn new_index_for_arrow_left() {
        init_env();
        let index = calculate_new_index_for_arrow(1, 6, 2, gdk::Key::Left);
        assert_eq!(index, index_left(1, 6));
    }

    #[test]
    fn new_index_for_arrow_right() {
        init_env();
        let index = calculate_new_index_for_arrow(0, 6, 2, gdk::Key::Right);
        assert_eq!(index, index_right(0, 6));
    }

    #[test]
    fn calculate_new_index_for_tab_forward() {
        init_env();
        let index = calculate_new_index_for_tab(2, 4, true);
        assert_eq!(index, 3);
    }

    #[test]
    fn calculate_new_index_for_tab_backward() {
        init_env();
        let index = calculate_new_index_for_tab(0, 4, false);
        assert_eq!(index, 3);
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
                            command: "echo default".to_string()
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
                            command: "echo test".to_string()
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

    #[test]
    fn test_select_css_path_valid_user_css() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let user_css_rel = "user_style.css";
        let user_css_path = tmp_dir.path().join(user_css_rel);
        fs::write(&user_css_path, "button { background-color: red; }")?;
        let default_css = tmp_dir.path().join("default.css");
        fs::write(&default_css, "button { background-color: blue; }")?;
        let result = select_css_path(
            Some(user_css_rel.to_string()),
            &config_path,
            &default_css,
            false,
        )?;
        assert_eq!(result, Some(user_css_path.clone()));
        Ok(())
    }

    #[test]
    fn test_select_css_path_neither_exist_returns_none() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp_dir.path().join("nonexistent_default.css");
        let result = select_css_path(user_css, &config_path, &default_css, false)?;
        assert_eq!(result, None);
        Ok(())
    }

    #[test]
    fn test_select_css_path_use_system_theme_true() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp_dir.path().join("default.css");
        fs::write(&default_css, "button { background-color: yellow; }")?;
        let result = select_css_path(user_css, &config_path, &default_css, true)?;
        assert_eq!(result, None);
        Ok(())
    }

    #[test]
    fn test_user_config_precedence_and_css_selection() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let tmp_home = tmp_dir.path().join("home");
        fs::create_dir_all(&tmp_home)?;
        env::set_var("HOME", &tmp_home);
        let user_config_dir = tmp_home.join(".config").join("fin");
        fs::create_dir_all(&user_config_dir)?;
        let user_config_content = r#"
    title = "Finë User Config"
    use_gtk_theme = false
    theme = "default"
    stylesheet = "user_style.css"

    [layout]
    window_width_ratio = 0.4
    window_height_ratio = 0.4
    button_font_ratio = 0.15

    [default_commands.default]
    columns = 2
    buttons = [
        { label = "Lock", command = "echo lock" }
    ]
    "#;
        let user_config_path = user_config_dir.join("config.toml");
        fs::write(&user_config_path, user_config_content)?;
        let user_css_path = user_config_dir.join("user_style.css");
        fs::write(&user_css_path, "button { background-color: #ff0000; }")?;
        let determined_path = determine_config_path();
        assert_eq!(
            determined_path, user_config_path,
            "The user configuration should take precedence."
        );
        let css_path = select_css_path(
            Some("user_style.css".to_string()),
            &user_config_path,
            &PathBuf::from(format!("{}/style.css", SYSTEM_CSS_DIR)),
            false,
        )?;
        assert_eq!(
            css_path,
            Some(user_css_path),
            "User-specified stylesheet should be resolved relative to the user config."
        );
        Ok(())
    }

    // Helper functions for build_ui tests.
    fn run_build_ui(config: Config, stylesheet_path: Option<PathBuf>) -> Result<()> {
        let app = Application::builder()
            .application_id("com.example.test.ui")
            .build();
        let build_result: Arc<Mutex<Option<Result<()>>>> = Arc::new(Mutex::new(None));

        {
            let build_result_clone = Arc::clone(&build_result);
            let config_for_closure = config.clone();
            app.connect_activate(move |app| {
                let res = build_ui(
                    app,
                    &config_for_closure,
                    stylesheet_path.clone(),
                    &config_for_closure.buttons,
                );
                if let Ok(mut guard) = build_result_clone.lock() {
                    *guard = Some(res);
                }
                app.quit();
            });
        }

        app.run();

        let mut guard = build_result
            .lock()
            .map_err(|_| anyhow!("Failed to lock build_result"))?;
        guard
            .take()
            .ok_or_else(|| anyhow!("No result from build_ui"))?
    }

    fn dummy_config(css_path: Option<String>, use_gtk_theme: bool) -> Config {
        Config {
            title: "Test UI".to_string(),
            columns: 2,
            buttons: vector![
                ButtonConfig {
                    label: "Test1".to_string(),
                    command: "echo test1".to_string(),
                },
                ButtonConfig {
                    label: "Test2".to_string(),
                    command: "echo test2".to_string(),
                },
                ButtonConfig {
                    label: "Test3".to_string(),
                    command: "echo test3".to_string(),
                }
            ],
            use_gtk_theme,
            css_path,
            default_commands: HashMap::new(),
            layout: None,
            theme: None,
        }
    }

    #[test]
    fn test_build_ui_with_valid_css() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            info!("Skipping test_build_ui_with_valid_css: no display available");
            return Ok(());
        }
        let tmp_dir = get_temp_dir()?;
        let valid_css = tmp_dir.path().join("valid_test.css");
        fs::write(&valid_css, "button { background-color: purple; }")?;
        let config = dummy_config(Some(valid_css.to_string_lossy().to_string()), false);
        let config_path = tmp_dir.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let default_css = tmp_dir.path().join("default.css");
        fs::write(&default_css, "button { background-color: blue; }")?;
        let stylesheet_path = select_css_path(
            config.css_path.clone(),
            &config_path,
            &default_css,
            config.use_gtk_theme,
        )?;
        assert_eq!(stylesheet_path, Some(valid_css.clone()));
        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_build_ui_with_default_css() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            info!("Skipping test_build_ui_with_default_css: no display available");
            return Ok(());
        }
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let config = dummy_config(Some("nonexistent.css".to_string()), false);
        let default_css = tmp_dir.path().join("default.css");
        fs::write(&default_css, "button { background-color: green; }")?;
        let stylesheet_path = select_css_path(
            config.css_path.clone(),
            &config_path,
            &default_css,
            config.use_gtk_theme,
        )?;
        assert_eq!(stylesheet_path, Some(default_css.clone()));
        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_build_ui_with_columns() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            info!("Skipping test_build_ui_with_columns: no display available");
            return Ok(());
        }
        let config = Config {
            title: "Column Test".to_string(),
            columns: 3,
            buttons: vector![
                ButtonConfig {
                    label: "1".into(),
                    command: "".into()
                },
                ButtonConfig {
                    label: "2".into(),
                    command: "".into()
                },
                ButtonConfig {
                    label: "3".into(),
                    command: "".into()
                },
                ButtonConfig {
                    label: "4".into(),
                    command: "".into()
                },
                ButtonConfig {
                    label: "5".into(),
                    command: "".into()
                },
            ],
            use_gtk_theme: false,
            css_path: None,
            default_commands: HashMap::new(),
            layout: None,
            theme: None,
        };
        let res = run_build_ui(config, None);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_build_ui_with_fallback_css() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            info!("Skipping test_build_ui_with_fallback_css: no display available");
            return Ok(());
        }
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let config = dummy_config(None, false);
        let default_css = tmp_dir.path().join("nonexistent_default.css");
        let stylesheet_path = select_css_path(
            config.css_path.clone(),
            &config_path,
            &default_css,
            config.use_gtk_theme,
        )?;
        assert_eq!(stylesheet_path, None);
        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());
        Ok(())
    }
}
