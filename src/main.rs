//! # HyprPower Logout Manager
//!
//! A GTK4-based logout manager for HyprPower. This application reads its configuration
//! from a TOML file, creates a grid-based UI with buttons for various commands, and supports
//! cyclic keyboard navigation. The design emphasizes functional programming principles,
//! using immutable data structures and pure functions where possible.
//!
//! Accessibility enhancements have been added by using widget labels and tooltips
//! to convey descriptive information for assistive technologies.

use anyhow::{anyhow, Context, Result};
use clap::parser::ValueSource;
use clap::{Arg, Command};
use glib::Propagation;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    gdk, Application, ApplicationWindow, Button, CssProvider, EventControllerFocus,
    EventControllerKey, Grid, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use im::Vector;
use im::{vector, HashMap};
use log::{error, info};
use serde::Deserialize;
use std::cell::Cell;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;
use std::rc::Rc;

/// Default number of columns to use if none is specified.
fn default_columns() -> usize {
    1
}

/// Deserialize a sequence into an `im::Vector<T>`.
fn deserialize_vector<'de, D, T>(deserializer: D) -> std::result::Result<Vector<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Clone,
{
    let vec = Vec::<T>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

/// Configuration for HyprPower loaded from a TOML file.
#[derive(Deserialize, Debug, Clone)]
struct Config {
    /// The title of the logout manager window.
    title: String,
    /// The number of columns in the UI grid.
    #[serde(default = "default_columns")]
    columns: usize,
    /// A list of button configurations.
    #[serde(default, deserialize_with = "deserialize_vector")]
    buttons: Vector<ButtonConfig>,
    /// Flag to indicate whether to use the system GTK theme.
    use_system_theme: bool,
    /// Desktop environment specific button overrides.
    #[serde(default)]
    de_overrides: HashMap<String, Vector<ButtonConfig>>,
    /// Default button commands keyed by desktop environment.
    #[serde(default)]
    default_commands: HashMap<String, Vector<ButtonConfig>>,
}

/// Configuration for an individual button.
#[derive(Deserialize, Debug, Clone)]
struct ButtonConfig {
    /// The label to display on the button.
    label: String,
    /// The command to execute when the button is clicked.
    command: String,
}

/// Returns the command set appropriate for the given desktop environment.
fn get_commands_for_de(de: &str, config: &Config) -> Vector<ButtonConfig> {
    if let Some(cmds) = config.de_overrides.get(de).filter(|cmds| !cmds.is_empty()) {
        return cmds.clone();
    }
    if let Some(cmds) = config
        .default_commands
        .get(de)
        .filter(|cmds| !cmds.is_empty())
    {
        return cmds.clone();
    }
    if let Some(cmds) = config
        .default_commands
        .get("default")
        .filter(|cmds| !cmds.is_empty())
    {
        return cmds.clone();
    }
    config.buttons.clone()
}

/// Calculates the grid layout as a nested vector of button indices.
fn calculate_layout(num_buttons: usize) -> std::result::Result<Vector<Vector<usize>>, String> {
    match num_buttons {
        1 => Ok(vector![vector![0]]),
        2 => Ok(vector![vector![0], vector![1]]),
        3 => Ok(vector![vector![0], vector![1], vector![2]]),
        4 => Ok(vector![vector![0, 1], vector![2, 3]]),
        5 => Ok(vector![
            vector![0],
            vector![1],
            vector![2],
            vector![3],
            vector![4]
        ]),
        6 => Ok(vector![vector![0, 1], vector![2, 3], vector![4, 5]]),
        7 => Err("7 buttons are not allowed, because your screen is not THAT wide".into()),
        8 => Ok(vector![
            vector![0, 1],
            vector![2, 3],
            vector![4, 5],
            vector![6, 7]
        ]),
        9 => Ok(vector![
            vector![0, 1, 2],
            vector![3, 4, 5],
            vector![6, 7, 8]
        ]),
        _ => Err("Number of buttons must be between 1 and 9.".into()),
    }
}

/// Calculates the new index for arrow-key navigation.
fn new_index_for_arrow(current: usize, total: usize, columns: usize, key: gdk::Key) -> usize {
    // Since `columns` is guaranteed to be > 0 (via default_columns), we can safely compute the remainder.
    match key {
        gdk::Key::Up | gdk::Key::KP_Up => {
            if current < columns {
                let remainder = total % columns;
                let last_row_start = if remainder == 0 {
                    total - columns
                } else {
                    total - remainder
                };
                (last_row_start + current).min(total - 1)
            } else {
                current - columns
            }
        }
        gdk::Key::Down | gdk::Key::KP_Down => {
            if current + columns >= total {
                current % columns
            } else {
                current + columns
            }
        }
        gdk::Key::Left | gdk::Key::KP_Left => {
            if current == 0 {
                total - 1
            } else {
                current - 1
            }
        }
        gdk::Key::Right | gdk::Key::KP_Right => {
            if current == total - 1 {
                0
            } else {
                current + 1
            }
        }
        _ => current,
    }
}

/// Calculates the new index for Tab-key navigation in a cyclic manner.
fn calculate_new_index_for_tab(index: usize, total: usize, forward: bool) -> usize {
    if forward {
        if index + 1 >= total {
            0
        } else {
            index + 1
        }
    } else if index == 0 {
        total - 1
    } else {
        index - 1
    }
}

/// Converts a vector of button configurations into a vector of GTK buttons.
fn create_buttons(app: &Application, btn_configs: &Vector<ButtonConfig>) -> Vector<Button> {
    btn_configs
        .iter()
        .map(|btn_cfg| create_action_button(app, &btn_cfg.label, &btn_cfg.command))
        .collect()
}

/// The main entry point of the application.
fn main() -> Result<()> {
    env_logger::init();

    let matches = Command::new("hyprpower")
        .version("0.1.0")
        .about("HyprPower Application")
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
        if matches.value_source("config").expect("checked contains_id") == ValueSource::CommandLine
        {
            info!("`config` set by user");
        } else {
            info!("`config` is defaulted");
        }
    }

    let config_path = matches
        .get_one::<String>("config")
        .map(|s| s.as_str())
        .unwrap_or("/usr/share/hyprpower/config.toml");
    let config = load_config(Path::new(config_path))?;
    let de = detect_desktop_environment();
    let commands = get_commands_for_de(&de, &config);
    let stylesheet_path = Path::new("/usr/share/hyprpower/style.css");

    let app = Application::builder()
        .application_id("com.hyprpower.launcher")
        .build();

    app.connect_activate(move |app| {
        if let Err(e) = build_ui(app, &config, stylesheet_path, &commands) {
            error!("Error building UI: {:?}", e);
            std::process::exit(1);
        }
    });

    app.run();
    Ok(())
}

/// Detects the current desktop environment from environment variables.
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

/// Loads the application configuration from the specified file.
fn load_config(path: &Path) -> Result<Config> {
    let config: Config = fs::read_to_string(path)
        .with_context(|| format!("Could not read config file {:?}", path))
        .and_then(|content| toml::from_str(&content).context("TOML deserialization error"))?;

    if config.columns == 0 {
        return Err(anyhow!(
            "Invalid configuration: columns must be greater than 0"
        ));
    }

    Ok(config)
}

/// Builds the user interface by creating the grid layout, buttons, and setting up event handlers.
fn build_ui(
    app: &Application,
    config: &Config,
    stylesheet_path: &Path,
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

    let window = ApplicationWindow::builder()
        .application(app)
        .title(&config.title)
        .default_width(600)
        .default_height(400)
        .build();
    window.set_decorated(false);
    window.set_transient_for(None::<&ApplicationWindow>);
    window.set_resizable(false);

    // Set a tooltip to describe the window (for accessibility).
    window.set_tooltip_text(Some("HyprPower logout manager window"));

    load_css(stylesheet_path, config.use_system_theme)?;

    let grid = create_grid();
    grid.set_tooltip_text(Some("Button grid"));

    // Determine grid layout.
    let layout = calculate_layout(buttons.len()).map_err(|e| anyhow!(e))?;

    // Create buttons from configuration.
    let all_buttons = create_buttons(app, buttons);

    // Arrange buttons into grid based on layout.
    let button_widgets: Vector<Button> = layout
        .iter()
        .enumerate()
        .flat_map(|(row, cols)| {
            let grid_clone = grid.clone();
            let all_buttons_clone = all_buttons.clone();
            cols.iter().enumerate().map(move |(col, &index)| {
                let button = all_buttons_clone.get(index).unwrap().clone();
                info!(
                    "Attaching button '{}' at row {}, col {}",
                    button.label().unwrap_or_default(),
                    row,
                    col
                );
                grid_clone.attach(&button, col as i32, row as i32, 1, 1);
                button
            })
        })
        .collect();

    setup_focus_chain(&grid, &button_widgets);
    let buttons_rc = Rc::new(button_widgets);

    window.set_child(Some(&grid));
    setup_focus_controller(&window, app);
    setup_key_handlers(&window, app, buttons_rc, config.columns);
    window.present();
    Ok(())
}

/// Creates and returns a new GTK grid container.
fn create_grid() -> Grid {
    Grid::builder()
        .column_homogeneous(true)
        .row_homogeneous(true)
        .column_spacing(10)
        .row_spacing(10)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build()
}

/// Sets the initial focus to the first button in the grid.
fn setup_focus_chain(grid: &Grid, buttons: &Vector<Button>) {
    if let Some(first_button) = buttons.get(0) {
        grid.set_focus_child(Some(first_button));
    }
}

/// Sets up a focus controller that quits the application when the window loses focus.
fn setup_focus_controller(window: &ApplicationWindow, app: &Application) {
    let app_clone = app.clone();
    let focus_controller = EventControllerFocus::new();
    focus_controller.connect_leave(move |_| {
        app_clone.quit();
    });
    window.add_controller(focus_controller);
}

/// Loads CSS styling from the specified file unless the system theme is used.
fn load_css(path: &Path, use_system_theme: bool) -> Result<()> {
    let provider = CssProvider::new();
    if !use_system_theme && path.exists() {
        let css_data = fs::read(path)
            .with_context(|| format!("Could not read CSS file '{}'", path.display()))?;
        let css_str = std::str::from_utf8(&css_data)
            .with_context(|| format!("CSS file '{}' is not valid UTF-8", path.display()))?;
        provider.load_from_string(css_str);
    } else {
        info!("Using system GTK4 theme");
    }
    let display = Display::default().ok_or_else(|| anyhow!("Could not get default display"))?;
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    Ok(())
}

/// Creates a GTK button with the specified label and command, setting accessibility properties.
/// When clicked, the button attempts to execute the command. On failure, an error is logged.
/// Error notifications are logged rather than shown visibly.
fn create_action_button(app: &Application, label: &str, command: &str) -> Button {
    let button = Button::with_label(label);
    // Rely on the button's label as its accessible name.
    // Set a tooltip to serve as an accessible description.
    button.set_tooltip_text(Some(&format!("Executes command: {}", command)));

    let command_string = command.to_string();
    let app_clone = app.clone();
    button.connect_clicked(move |_| {
        if !command_string.is_empty() {
            if let Err(e) = ProcessCommand::new("sh")
                .arg("-c")
                .arg(&command_string)
                .spawn()
                .with_context(|| format!("Failed to execute command '{}'", command_string))
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

/// Logs an error notification (no visible dialog is shown).
fn show_error_dialog(_app: &Application, message: &str) {
    error!("Error Notification: {}", message);
}

/// Sets up key event handlers for cyclic navigation of the buttons.
/// Supports arrow keys and Tab (forward and backward) for changing focus.
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
    {
        let app = app.clone();
        let buttons = buttons.clone();
        let current_index = current_index.clone();
        controller.connect_key_pressed(move |_, keyval, _hardware_keycode, _state| {
            let total = buttons.len();
            let current = current_index.get();
            let new_index = match keyval {
                gdk::Key::Escape => {
                    app.quit();
                    return Propagation::Stop;
                }
                gdk::Key::Return => {
                    if let Some(button) = buttons.get(current) {
                        button.emit_clicked();
                    }
                    return Propagation::Stop;
                }
                gdk::Key::Tab => calculate_new_index_for_tab(current, total, true),
                gdk::Key::ISO_Left_Tab => calculate_new_index_for_tab(current, total, false),
                key => new_index_for_arrow(current, total, columns, key),
            };

            if new_index != current {
                current_index.set(new_index);
                if let Some(button) = buttons.get(new_index) {
                    button.grab_focus();
                }
            }
            Propagation::Stop
        });
    }
    window.add_controller(controller);
}
