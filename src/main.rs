//! A GTK4 launcher application configured via a TOML file.
//!
//! This application displays a grid of buttons as defined in a configuration file.
//! Each button can execute a specified command when clicked. The UI supports arrow-key,
//! tab-based keyboard navigation, and custom styling via CSS.

use anyhow::{Context, Result};
use glib::Propagation;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    gdk, Application, ApplicationWindow, Button, CssProvider, EventControllerKey, Grid,
    STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use im::Vector;
use serde::Deserialize;
use std::cell::Cell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;

/// Deserializes a vector into an `im::Vector`.
fn deserialize_vector<'de, D, T>(deserializer: D) -> std::result::Result<Vector<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Clone,
{
    let vec = Vec::<T>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

/// Configuration for the launcher application.
#[derive(Deserialize, Debug)]
struct Config {
    /// Window title.
    title: String,
    /// Number of columns in the button grid.
    columns: usize,
    /// Button configurations.
    #[serde(deserialize_with = "deserialize_vector")]
    buttons: Vector<ButtonConfig>,
    /// Path to the CSS stylesheet.
    stylesheet: String,
}

/// Configuration for an individual button.
#[derive(Deserialize, Debug, Clone)]
struct ButtonConfig {
    /// Label displayed on the button (text or icon glyph).
    label: String,
    /// Shell command executed when the button is clicked.
    command: String,
}

fn main() {
    // Exit with an appropriate code on error
    std::process::exit(match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            1
        }
    });
}

/// Main entry point, wraps up error handling via anyhow.
fn run() -> Result<()> {
    // Gather arguments
    let args: Vec<String> = env::args().collect();
    // or use `im::Vector` if you prefer

    // 1) If user passed a path, use that
    // 2) else if LAUNCHER_CONFIG_PATH is set, use that
    // 3) else fallback to find_config_path()
    let config_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else if let Ok(env_path) = env::var("LAUNCHER_CONFIG_PATH") {
        PathBuf::from(env_path)
    } else {
        find_config_path()
    };

    // Load config
    let config = load_config(&config_path)
        .with_context(|| format!("Failed to load configuration from {:?}", config_path))?;

    // Create the GTK application
    let app = Application::builder()
        .application_id("com.hyprpower.launcher")
        .build();

    // When the application is activated, build the UI
    app.connect_activate(move |app| {
        if let Err(e) = build_ui(app, &config) {
            eprintln!("Error building UI: {:?}", e);
            std::process::exit(1);
        }
    });

    // Launch the application
    app.run();
    Ok(())
}

/// Returns the configuration path, checking user-specific and system-wide locations.
fn find_config_path() -> PathBuf {
    // 1. Check user config in ~/.config/hyprpower/config.toml
    if let Ok(xdg_config_home) = env::var("XDG_CONFIG_HOME") {
        let user_config = Path::new(&xdg_config_home)
            .join("hyprpower")
            .join("config.toml");
        if user_config.exists() {
            return user_config;
        }
    } else if let Ok(home) = env::var("HOME") {
        let user_config = Path::new(&home)
            .join(".config")
            .join("hyprpower")
            .join("config.toml");
        if user_config.exists() {
            return user_config;
        }
    }

    // 2. Fallback to system-wide default: /usr/share/hyprlauncher/config.toml
    Path::new("/usr/share/hyprlauncher").join("config.toml")
}

/// Loads and parses the configuration file from the given path.
fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Could not read config file {:?}", path))?;

    toml::from_str(&content).context("TOML deserialization error")
}

/// Creates a button with the given label and command.
fn create_action_button(app: &Application, label: &str, command: &str) -> Button {
    let button = Button::with_label(label);
    let command_string = command.to_string();
    let app = app.clone();

    // When the button is clicked, run the command in a shell, then quit the app
    button.connect_clicked(move |_| {
        if !command_string.is_empty() {
            if let Err(e) = Command::new("sh").arg("-c").arg(&command_string).spawn() {
                eprintln!("Failed to execute command '{}': {}", command_string, e);
            }
        }
        app.quit();
    });

    button
}

/// Builds the primary UI: window, grid layout, and buttons.
fn build_ui(app: &Application, config: &Config) -> Result<()> {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&config.title)
        .default_width(600)
        .default_height(400)
        .build();

    // Hide window decorations (title bar, etc.)
    window.set_decorated(false);
    // Make it non-resizable
    window.set_resizable(false);

    // Load the CSS stylesheet
    load_css(&config.stylesheet)?;

    // Create a grid layout where each column & row is homogeneous
    let grid = Grid::builder()
        .column_homogeneous(true)
        .row_homogeneous(true)
        .column_spacing(10)
        .row_spacing(10)
        .margin_top(20)
        .margin_bottom(20)
        .margin_start(20)
        .margin_end(20)
        .build();

    // Attach each button in row-major order
    for (index, button_config) in config.buttons.iter().enumerate() {
        let button = create_action_button(app, &button_config.label, &button_config.command);
        let col = (index % config.columns) as i32;
        let row = (index / config.columns) as i32;
        grid.attach(&button, col, row, 1, 1);
    }

    // Add the grid to the window
    window.set_child(Some(&grid));
    window.present();

    // Collect all buttons to set up custom keyboard navigation
    let children: Vector<_> =
        std::iter::successors(grid.first_child(), |child| child.next_sibling()).collect();
    let buttons: Vector<Button> = children
        .into_iter()
        .filter_map(|w| w.downcast::<Button>().ok())
        .collect();

    setup_key_handlers(&window, app, buttons, config.columns);
    Ok(())
}

/// Loads the CSS stylesheet from the given path and applies it to the application.
fn load_css(css_path: &str) -> Result<()> {
    let provider = CssProvider::new();
    let css_data =
        fs::read(css_path).with_context(|| format!("Could not read CSS file '{}'", css_path))?;

    let css_str = std::str::from_utf8(&css_data)
        .with_context(|| format!("CSS file '{}' is not valid UTF-8", css_path))?;

    provider.load_from_data(css_str);

    let display =
        Display::default().ok_or_else(|| anyhow::anyhow!("Could not get default display"))?;

    // Apply the CSS to the app
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    Ok(())
}

/// Sets up key event handling for navigation in a grid of buttons.
fn setup_key_handlers(
    window: &ApplicationWindow,
    app: &Application,
    buttons: Vector<Button>,
    columns: usize,
) {
    // Let the window accept focus and request it
    window.set_can_focus(true);
    window.grab_focus();

    // Track the current focused button index
    let current_index = Rc::new(Cell::new(0));

    // Focus the first button if available
    if let Some(first_button) = buttons.get(0) {
        first_button.grab_focus();
    }

    // Create a key event controller
    let controller = EventControllerKey::new();

    controller.connect_key_pressed({
        let app = app.clone();
        let buttons = buttons.clone();
        let current_index = current_index.clone();

        move |_, keyval, _hardware_keycode, state| {
            let total_buttons = buttons.len();
            let index = current_index.get();

            let new_index = match keyval {
                gdk::Key::Escape => {
                    // Quit the entire app
                    app.quit();
                    return Propagation::Stop;
                }
                gdk::Key::Return => {
                    // Activate (click) the current button
                    if let Some(button) = buttons.get(index) {
                        button.emit_clicked();
                    }
                    return Propagation::Stop;
                }
                // Arrow keys / numeric keypad arrows
                gdk::Key::Up | gdk::Key::KP_Up => index.saturating_sub(columns),
                gdk::Key::Down | gdk::Key::KP_Down => {
                    (index + columns).min(total_buttons.saturating_sub(1))
                }
                gdk::Key::Left | gdk::Key::KP_Left => index.saturating_sub(1),
                gdk::Key::Right | gdk::Key::KP_Right => {
                    (index + 1).min(total_buttons.saturating_sub(1))
                }
                // Tab and Shift+Tab
                gdk::Key::Tab => {
                    let shift_pressed = state.contains(gdk::ModifierType::SHIFT_MASK);
                    if shift_pressed {
                        // Move backward
                        if index == 0 {
                            total_buttons.saturating_sub(1)
                        } else {
                            index.saturating_sub(1)
                        }
                    } else {
                        // Move forward
                        (index + 1) % total_buttons
                    }
                }
                // Some systems use ISO_Left_Tab for Shift+Tab
                gdk::Key::ISO_Left_Tab => {
                    if index == 0 {
                        total_buttons.saturating_sub(1)
                    } else {
                        index.saturating_sub(1)
                    }
                }
                // If it doesn't match any recognized key, let GTK handle it
                _ => return Propagation::Proceed,
            };

            // If we have a new index, focus the corresponding button
            if new_index != index {
                current_index.set(new_index);
                if let Some(button) = buttons.get(new_index) {
                    button.grab_focus();
                }
            }

            // Stop propagation since we handled it
            Propagation::Stop
        }
    });

    // Add the controller to the window
    window.add_controller(controller);
}
