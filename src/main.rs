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
    gdk, prelude::*, Application, ApplicationWindow, Button, CssProvider, EventControllerFocus,
    EventControllerKey, Grid,
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

// ------------------------------------------------------------
// Constants for grid spacing and margins (avoiding magic numbers)
// ------------------------------------------------------------
const GRID_COLUMN_SPACING: i32 = 10;
const GRID_ROW_SPACING: i32 = 10;
const GRID_MARGIN: i32 = 20;

// ------------------------------------------------------------
// 1. Configuration & CSS Resolution Helpers
// ------------------------------------------------------------

/// Determines the configuration file path by checking, in order:
/// 1. The path specified by the `XDG_CONFIG_HOME` environment variable.
/// 2. The path derived from the `HOME` environment variable (`$HOME/.config/fin/config.toml`).
/// 3. Falls back to the system-wide configuration file at `/usr/share/fin/config.toml`.
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
        .unwrap_or(PathBuf::from("/usr/share/fin/config.toml"))
}

/// Resolves the CSS file path using these precedence rules:
///
/// 1. When `use_gtk_theme` is true:
///    - Ignores custom CSS configurations
///    - Returns `None` to apply the system GTK theme
///
/// 2. When user provides `config_css`:
///    - Resolves path relative to `config_path`
///    - Uses file if found
///    - Logs warning and falls back to `default_css` if missing
///
/// 3. With no user-provided CSS:
///    - Uses `default_css` path directly
///
/// 4. If `default_css` missing:
///    - Attempts to find alternative in system configuration
///
/// 5. When all options fail:
///    - Returns `None` to use GTK4's built-in styling
///
/// # Parameters
/// - `config_css`: User-provided CSS path from configuration (optional)
/// - `config_path`: Parent directory for resolving relative CSS paths
/// - `default_css`: Fallback CSS path bundled with application
/// - `use_gtk_theme`: When true, prioritizes system theme over custom CSS
///
/// # Returns
/// `Result<Option<PathBuf>, Error>` where:
/// - `Ok(Some(path))`: Valid CSS file found
/// - `Ok(None)`: Use default GTK styling
/// - `Err(e)`: File system error occurred during resolution
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

    let css_path = config_css
        .map(|user_css| {
            let user_path = PathBuf::from(&user_css);
            let resolved_path = if user_path.is_absolute() {
                user_path.clone()
            } else {
                config_path
                    .parent()
                    .map_or(user_path.clone(), |parent| parent.join(&user_path))
            };
            info!("Resolved CSS path: {:?}", resolved_path);
            if resolved_path.exists() {
                resolved_path
            } else {
                warn!(
                    "User provided CSS '{}' not found. Falling back to default CSS '{}'.",
                    resolved_path.display(),
                    default_css.display()
                );
                default_css.to_path_buf()
            }
        })
        .unwrap_or(default_css.to_path_buf());

    if css_path.exists() {
        info!("Using CSS file at '{}'.", css_path.display());
        Ok(Some(css_path))
    } else {
        info!("Default CSS not found. Checking system config.");
        let system_config_path = env::var("FIN_SYSTEM_CONFIG")
            .unwrap_or_else(|_| "/usr/share/fin/config.toml".to_string());
        let system_config_content = fs::read_to_string(&system_config_path).ok();
        let system_config = system_config_content
            .and_then(|content| toml::from_str::<SystemConfig>(&content).ok())
            .and_then(|config| config.css_path);

        let system_css = system_config
            .map(|css_str| {
                let config_dir = Path::new("/usr/share/fin");
                let path = config_dir.join(css_str);
                if path.exists() {
                    info!("Using system-configured CSS at '{}'.", path.display());
                    Some(path)
                } else {
                    warn!("System-configured CSS '{}' does not exist.", path.display());
                    None
                }
            })
            .unwrap_or(None);
        if system_css.is_none() {
            warn!("No valid CSS file found. Falling back to GTK built-in style.");
        }
        Ok(system_css)
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
    let system_theme_path =
        PathBuf::from("/usr/share/fin/themes").join(format!("{}.toml", theme_name));
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

// ------------------------------------------------------------
// 2. Configuration Structures and Loading
// ------------------------------------------------------------

/// Returns the default number of columns if none is specified.
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
    buttons: Vector<ButtonConfig>,
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
    0.3
}
/// Returns the default window height ratio.
fn default_window_height_ratio() -> f64 {
    0.3
}
/// Returns the default button font ratio.
fn default_button_font_ratio() -> f64 {
    0.14
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
    buttons: Vector<ButtonConfig>,
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

/// Loads and parses the configuration file located at `path`.
///
/// # Parameters
///
/// - `path`: The path to the configuration file.
///
/// # Returns
///
/// A `Result` containing the parsed `Config` or an error with context including the file path.
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
/// If the desktop environment is not specifically configured, falls back to the default commands.
///
/// # Parameters
///
/// - `de`: The detected desktop environment.
/// - `config`: The overall configuration.
///
/// # Returns
///
/// A tuple `(buttons, columns)`.
fn get_commands_for_de(de: &str, config: &Config) -> (Vector<ButtonConfig>, usize) {
    if let Some(de_cmd) = config.default_commands.get(de) {
        (de_cmd.buttons.clone(), de_cmd.columns)
    } else if let Some(default_cmd) = config.default_commands.get("default") {
        (default_cmd.buttons.clone(), default_cmd.columns)
    } else {
        (config.buttons.clone(), config.columns)
    }
}

// ------------------------------------------------------------
// 3. UI and Navigation Functions
// ------------------------------------------------------------

/// Creates an action button with the specified label and command.
/// The button is styled with the "action-button" CSS class and shows a tooltip with the command.
///
/// # Parameters
///
/// - `app`: Reference to the GTK application.
/// - `label`: The label for the button.
/// - `command`: The command to execute when the button is clicked.
///
/// # Returns
///
/// A `Button` widget.
fn create_action_button(app: &Application, label: &str, command: &str) -> Button {
    let button = Button::with_label(label);
    button.add_css_class("action-button");
    button.set_tooltip_text(Some(&format!("Executes command: {}", command)));
    let command_string = command.to_string();
    let app_clone = app.clone();
    button.connect_clicked(move |_| {
        if !command_string.is_empty() {
            if let Err(e) = ProcessCommand::new("sh")
                .arg("-c")
                .arg(&command_string)
                .spawn()
                .with_context(|| {
                    format!(
                        "Failed to execute command '{}' on button click",
                        command_string
                    )
                })
            {
                let err_msg = format!("Failed to execute command '{}': {}", command_string, e);
                error!("{}", err_msg);
                show_error_dialog(&app_clone, &err_msg);
            }
        }
        app_clone.quit();
    });
    button
}

/// Displays an error message (currently by logging it).
fn show_error_dialog(_app: &Application, message: &str) {
    error!("Error Notification: {}", message);
}

/// Sets up the focus chain by setting the first button as the focus child in the grid.
fn setup_focus_chain(grid: &Grid, buttons: &Vector<Button>) {
    if let Some(first_button) = buttons.get(0) {
        grid.set_focus_child(Some(first_button));
    }
}

/// Sets up a focus controller for the window so that leaving focus causes the application to quit.
fn setup_focus_controller(window: &ApplicationWindow, app: &Application) {
    let app_clone = app.clone();
    let focus_controller = EventControllerFocus::new();
    focus_controller.connect_leave(move |_| {
        app_clone.quit();
    });
    window.add_controller(focus_controller);
}

/// Computes a new index for button focus when an arrow key is pressed.
///
/// # Parameters
///
/// - `current`: The current focused button index.
/// - `total`: The total number of buttons.
/// - `columns`: The number of columns in the grid.
/// - `key`: The key that was pressed.
///
/// # Returns
///
/// The new index after applying the arrow key movement.
fn new_index_for_arrow(current: usize, total: usize, columns: usize, key: gdk::Key) -> usize {
    match key {
        gdk::Key::Up | gdk::Key::KP_Up => current
            .checked_sub(columns)
            .unwrap_or_else(|| {
                let last_row_start =
                    total.saturating_sub(columns) - (total % columns).saturating_sub(1);
                last_row_start + current % columns
            })
            .min(total.saturating_sub(1)),
        gdk::Key::Down | gdk::Key::KP_Down => {
            let next_row = current + columns;
            if next_row < total {
                next_row
            } else {
                current % columns
            }
        }
        gdk::Key::Left | gdk::Key::KP_Left => (current + total - 1) % total,
        gdk::Key::Right | gdk::Key::KP_Right => (current + 1) % total,
        _ => current,
    }
}

/// Computes a new index for button focus when the Tab key is pressed.
///
/// # Parameters
///
/// - `index`: The current index.
/// - `total`: The total number of buttons.
/// - `forward`: If true, moves forward; if false, moves backward.
///
/// # Returns
///
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
///
/// - `window`: The application window.
/// - `app`: The GTK application.
/// - `buttons`: A reference-counted vector of buttons.
/// - `columns`: The number of columns in the grid.
fn setup_key_handlers(
    window: &ApplicationWindow,
    app: &Application,
    buttons: Rc<Vector<Button>>,
    columns: usize,
) {
    window.set_can_focus(true);
    window.grab_focus();
    if let Some(first_button) = buttons.get(0) {
        first_button.grab_focus();
    }
    let current_index = Rc::new(Cell::new(0));
    let controller = EventControllerKey::new();

    // Clone necessary references.
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
            key => new_index_for_arrow(current, total, columns, key),
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
///
/// - `app`: The GTK application.
/// - `buttons`: A vector of button configurations.
/// - `columns`: The number of columns in the grid.
///
/// # Returns
///
/// A `Result` containing a tuple of the `Grid` widget and a vector of created `Button` widgets.
fn compose_grid(
    app: &Application,
    buttons: &Vector<ButtonConfig>,
    columns: usize,
) -> Result<(Grid, Vector<Button>)> {
    let grid = create_grid();
    let all_buttons = buttons
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
        .collect::<Vector<_>>();
    Ok((grid, all_buttons))
}

/// Creates a new `Grid` widget with default spacing and margins.
///
/// # Returns
///
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
///
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

// ------------------------------------------------------------
// 4. Theme Loading
// ------------------------------------------------------------

/// Represents a theme with color palette and styling definitions.
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
///
/// - `path`: The path to the theme file.
///
/// # Returns
///
/// A `Result` containing the loaded `Theme` or an error with context.
fn load_theme<P: AsRef<Path>>(path: P) -> Result<Theme> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Could not read theme file at path: {:?}", path.as_ref()))?;
    let theme: Theme = toml::from_str(&content)
        .with_context(|| format!("Theme deserialization error in file: {:?}", path.as_ref()))?;
    Ok(theme)
}

// ------------------------------------------------------------
// 5. Building the UI
// ------------------------------------------------------------

/// Builds the application UI by combining dynamic CSS, user CSS, and theme CSS.
/// It creates the main window, applies the combined CSS, and sets up the grid layout and key handlers.
///
/// # Parameters
///
/// - `app`: The GTK application.
/// - `config`: The application configuration.
/// - `stylesheet_path`: An optional path to the user-provided stylesheet.
/// - `buttons`: A vector of button configurations to display.
///
/// # Returns
///
/// A `Result` indicating success or failure.
fn build_ui(
    app: &Application,
    config: &Config,
    stylesheet_path: Option<PathBuf>,
    buttons: &Vector<ButtonConfig>,
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
        window_width_ratio: default_window_width_ratio(),
        window_height_ratio: default_window_height_ratio(),
        button_font_ratio: default_button_font_ratio(),
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

    // Create dynamic CSS for responsive button font size.
    let dynamic_css = format!(
        ".action-button {{ font-size: {}px; }}",
        (window_height as f64 * layout.button_font_ratio) as i32
    );

    // Conditionally read user CSS (unless using the GTK theme).
    let user_css = (!config.use_gtk_theme)
        .then(|| {
            stylesheet_path
                .as_ref()
                .and_then(|path| fs::read_to_string(path).ok())
                .unwrap_or_default()
        })
        .unwrap_or_default();

    // Get theme CSS via the helper function.
    let theme_css = get_theme_css(config);

    // Combine all CSS rules. The order determines precedence (later rules override earlier ones).
    let combined_css = format!("{}\n{}\n{}", dynamic_css, user_css, theme_css);

    // Create one CSS provider and load the combined CSS.
    let provider = CssProvider::new();
    provider.load_from_string(&combined_css);
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Build the UI grid and attach buttons.
    let (grid, all_buttons) = compose_grid(app, buttons, config.columns)?;
    setup_focus_chain(&grid, &all_buttons);
    let buttons_rc = Rc::new(all_buttons);

    window.set_child(Some(&grid));
    setup_focus_controller(&window, app);
    setup_key_handlers(&window, app, buttons_rc, config.columns);
    window.present();

    Ok(())
}

// ------------------------------------------------------------
// 6. Main Entry Point
// ------------------------------------------------------------

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

    let config = load_config(&config_path)?;
    let de = detect_desktop_environment();
    let (commands, de_columns) = get_commands_for_de(&de, &config);
    let config = Config {
        columns: de_columns,
        ..config
    };

    let default_css = PathBuf::from("/usr/share/fin/style.css");

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

// ------------------------------------------------------------
// Tests
// ------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use im::{hashmap, vector};
    use log::info;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, Once};

    static INIT: Once = Once::new();
    /// Initializes the test environment and logger.
    fn init_env() {
        INIT.call_once(|| {
            let _ = env_logger::builder().is_test(true).try_init();
            env::set_var("FIN_SYSTEM_CONFIG", "/nonexistent/path/config.toml");
        });
    }

    /// Returns a dedicated temporary directory for tests.
    fn get_temp_dir() -> Result<PathBuf> {
        let mut tmp = env::temp_dir();
        tmp.push("fin_test");
        fs::create_dir_all(&tmp)?;
        Ok(tmp)
    }

    /// Helper function to remove a file, ignoring errors.
    fn remove_test_file(path: &Path) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_config_valid_file() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.join("valid_config.toml");
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
        // Use unique filenames per iteration if running in a loop
        let config_path = tmp_dir.join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;

        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp_dir.join("default.css");
        fs::write(&default_css, "button { background-color: green; }")?;

        info!("Config path: {:?}", config_path);
        info!("Default CSS path: {:?}", default_css);

        let result = select_css_path(user_css, &config_path, &default_css, false)?;
        info!("Selected CSS path: {:?}", result);
        assert_eq!(result, Some(default_css.clone()));

        remove_test_file(&default_css);
        remove_test_file(&config_path);
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
        let index = new_index_for_arrow(3, 6, 2, gdk::Key::Up);
        assert_eq!(index, 1);
    }

    #[test]
    fn new_index_for_arrow_down() {
        init_env();
        let index = new_index_for_arrow(1, 6, 2, gdk::Key::Down);
        assert_eq!(index, 3);
    }

    #[test]
    fn new_index_for_arrow_left() {
        init_env();
        let index = new_index_for_arrow(1, 6, 2, gdk::Key::Left);
        assert_eq!(index, 0);
    }

    #[test]
    fn new_index_for_arrow_right() {
        init_env();
        let index = new_index_for_arrow(0, 6, 2, gdk::Key::Right);
        assert_eq!(index, 1);
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
        let config_path = tmp_dir.join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;

        let user_css_rel = "user_style.css";
        let user_css_path = tmp_dir.join(user_css_rel);
        fs::write(&user_css_path, "button { background-color: red; }")?;

        let default_css = tmp_dir.join("default.css");
        fs::write(&default_css, "button { background-color: blue; }")?;

        let result = select_css_path(
            Some(user_css_rel.to_string()),
            &config_path,
            &default_css,
            false,
        )?;
        assert_eq!(result, Some(user_css_path.clone()));

        remove_test_file(&user_css_path);
        remove_test_file(&default_css);
        remove_test_file(&config_path);
        Ok(())
    }

    #[test]
    fn test_select_css_path_neither_exist_returns_none() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;

        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp_dir.join("nonexistent_default.css");

        let result = select_css_path(user_css, &config_path, &default_css, false)?;
        assert_eq!(result, None);

        remove_test_file(&config_path);
        Ok(())
    }

    #[test]
    fn test_select_css_path_use_system_theme_true() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;

        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp_dir.join("default.css");
        fs::write(&default_css, "button { background-color: yellow; }")?;

        let result = select_css_path(user_css, &config_path, &default_css, true)?;
        assert_eq!(result, None);

        remove_test_file(&default_css);
        remove_test_file(&config_path);
        Ok(())
    }

    #[test]
    fn test_user_config_precedence_and_css_selection() -> Result<()> {
        init_env();
        let tmp_home = get_temp_dir()?.join("home");
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
            &PathBuf::from("/usr/share/fin/style.css"),
            false,
        )?;
        assert_eq!(
            css_path,
            Some(user_css_path),
            "User-specified stylesheet should be resolved relative to the user config."
        );

        Ok(())
    }

    fn create_file(path: &Path, content: &str) -> Result<()> {
        fs::write(path, content)?;
        Ok(())
    }

    fn remove_file(path: &Path) {
        let _ = fs::remove_file(path);
    }

    fn run_build_ui(config: Config, stylesheet_path: Option<PathBuf>) -> Result<()> {
        let app = Application::builder()
            .application_id("com.example.test.ui")
            .build();
        let build_result: Arc<Mutex<Option<Result<()>>>> = Arc::new(Mutex::new(None));
        let build_result_clone = build_result.clone();
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
        app.run();
        {
            let mut guard = build_result
                .lock()
                .map_err(|_| anyhow!("Failed to lock build_result"))?;
            guard
                .take()
                .ok_or_else(|| anyhow!("build_result was not set"))?
        }
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
        let tmp_dir = env::temp_dir();
        let valid_css = tmp_dir.join("valid_test.css");
        create_file(&valid_css, "button { background-color: purple; }")?;

        let config = dummy_config(Some(valid_css.to_string_lossy().to_string()), false);
        let config_path = tmp_dir.join("dummy_config.toml");
        create_file(&config_path, "dummy config")?;
        let default_css = tmp_dir.join("default.css");
        create_file(&default_css, "button { background-color: blue; }")?;

        let stylesheet_path = select_css_path(
            config.css_path.clone(),
            &config_path,
            &default_css,
            config.use_gtk_theme,
        )?;
        assert_eq!(stylesheet_path, Some(valid_css.clone()));

        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());

        remove_file(&valid_css);
        remove_file(&default_css);
        remove_file(&config_path);
        Ok(())
    }

    #[test]
    fn test_build_ui_with_default_css() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            info!("Skipping test_build_ui_with_default_css: no display available");
            return Ok(());
        }
        let tmp_dir = env::temp_dir();
        let config_path = tmp_dir.join("dummy_config.toml");
        create_file(&config_path, "dummy config")?;

        let config = dummy_config(Some("nonexistent.css".to_string()), false);
        let default_css = tmp_dir.join("default.css");
        create_file(&default_css, "button { background-color: green; }")?;

        let stylesheet_path = select_css_path(
            config.css_path.clone(),
            &config_path,
            &default_css,
            config.use_gtk_theme,
        )?;
        assert_eq!(stylesheet_path, Some(default_css.clone()));

        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());

        remove_file(&default_css);
        remove_file(&config_path);
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
        let tmp_dir = env::temp_dir();
        let config_path = tmp_dir.join("dummy_config.toml");
        create_file(&config_path, "dummy config")?;

        let config = dummy_config(None, false);
        let default_css = tmp_dir.join("nonexistent_default.css");

        let stylesheet_path = select_css_path(
            config.css_path.clone(),
            &config_path,
            &default_css,
            config.use_gtk_theme,
        )?;
        assert_eq!(stylesheet_path, None);

        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());

        remove_file(&config_path);
        Ok(())
    }
}
