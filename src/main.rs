use anyhow::{anyhow, Context, Result};
use glib::Propagation;
use gtk4::gdk::Display;
use gtk4::prelude::*;
use gtk4::{
    gdk, Application, ApplicationWindow, Button, CssProvider, EventControllerFocus,
    EventControllerKey, Grid, STYLE_PROVIDER_PRIORITY_APPLICATION,
};
use im::Vector;
use serde::Deserialize;
use std::cell::Cell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn deserialize_vector<'de, D, T>(deserializer: D) -> std::result::Result<Vector<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Clone,
{
    let vec = Vec::<T>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

#[derive(Deserialize, Debug)]
struct Config {
    title: String,
    columns: usize,
    #[serde(deserialize_with = "deserialize_vector")]
    buttons: Vector<ButtonConfig>,
    stylesheet: String,
    use_system_theme: bool, // New field to switch between custom and system theme
}

#[derive(Deserialize, Debug, Clone)]
struct ButtonConfig {
    label: String,
    command: String,
}

fn main() {
    std::process::exit(match run() {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {:?}", e);
            1
        }
    });
}

fn run() -> Result<()> {
    let args: Vector<String> = env::args().collect();
    let config_path = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else if let Ok(env_path) = env::var("LAUNCHER_CONFIG_PATH") {
        PathBuf::from(env_path)
    } else {
        find_config_path()
    };
    let config = load_config(&config_path)
        .with_context(|| format!("Failed to load configuration from {:?}", config_path))?;
    let stylesheet_path = resolve_stylesheet_path(&config_path, &config.stylesheet);
    let app = Application::builder()
        .application_id("com.hyprpower.launcher")
        .build();
    app.connect_activate(move |app| {
        if let Err(e) = build_ui(app, &config, &stylesheet_path) {
            eprintln!("Error building UI: {:?}", e);
            std::process::exit(1);
        }
    });
    app.run();
    Ok(())
}

fn find_config_path() -> PathBuf {
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
    Path::new("/usr/share/hyprpower").join("config.toml")
}

fn load_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Could not read config file {:?}", path))?;
    toml::from_str(&content).context("TOML deserialization error")
}

fn resolve_stylesheet_path(config_file_path: &Path, stylesheet: &str) -> PathBuf {
    let stylesheet_path = Path::new(stylesheet);
    if stylesheet_path.is_absolute() {
        stylesheet_path.to_path_buf()
    } else {
        config_file_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(stylesheet_path)
    }
}

fn build_ui(app: &Application, config: &Config, stylesheet_path: &Path) -> Result<()> {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(&config.title)
        .default_width(600)
        .default_height(400)
        .build();
    window.set_decorated(false);
    window.set_transient_for(None::<&ApplicationWindow>);
    window.set_resizable(false);
    load_css(stylesheet_path, config.use_system_theme)?;
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
    for (index, button_config) in config.buttons.iter().enumerate() {
        let button = create_action_button(app, &button_config.label, &button_config.command);
        let col = (index % config.columns) as i32;
        let row = (index / config.columns) as i32;
        grid.attach(&button, col, row, 1, 1);
    }
    window.set_child(Some(&grid));
    let app_clone = app.clone();
    let focus_controller = EventControllerFocus::new();
    focus_controller.connect_leave(move |_| {
        app_clone.quit();
    });
    window.add_controller(focus_controller);
    let children: Vector<_> =
        std::iter::successors(grid.first_child(), |child| child.next_sibling()).collect();
    let buttons: Vector<Button> = children
        .into_iter()
        .filter_map(|w| w.downcast::<Button>().ok())
        .collect();
    setup_key_handlers(&window, app, buttons, config.columns);
    window.present();
    Ok(())
}

fn load_css(path: &Path, use_system_theme: bool) -> Result<()> {
    let provider = CssProvider::new();
    if !use_system_theme && path.exists() {
        let css_data = fs::read(path)
            .with_context(|| format!("Could not read CSS file '{}'", path.display()))?;
        let css_str = std::str::from_utf8(&css_data)
            .with_context(|| format!("CSS file '{}' is not valid UTF-8", path.display()))?;
        provider.load_from_data(css_str);
    } else {
        eprintln!("Using system GTK4 theme");
    }
    let display = Display::default().ok_or_else(|| anyhow!("Could not get default display"))?;
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    Ok(())
}

fn create_action_button(app: &Application, label: &str, command: &str) -> Button {
    let button = Button::with_label(label);
    let command_string = command.to_string();
    let app = app.clone();
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

fn setup_key_handlers(
    window: &ApplicationWindow,
    app: &Application,
    buttons: Vector<Button>,
    columns: usize,
) {
    window.set_can_focus(true);
    window.grab_focus();
    let current_index = std::rc::Rc::new(Cell::new(0));
    if let Some(first_button) = buttons.get(0) {
        first_button.grab_focus();
    }
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
                    app.quit();
                    return Propagation::Stop;
                }
                gdk::Key::Return => {
                    if let Some(button) = buttons.get(index) {
                        button.emit_clicked();
                    }
                    return Propagation::Stop;
                }
                gdk::Key::Up | gdk::Key::KP_Up => index.saturating_sub(columns),
                gdk::Key::Down | gdk::Key::KP_Down => {
                    (index + columns).min(total_buttons.saturating_sub(1))
                }
                gdk::Key::Left | gdk::Key::KP_Left => index.saturating_sub(1),
                gdk::Key::Right | gdk::Key::KP_Right => {
                    (index + 1).min(total_buttons.saturating_sub(1))
                }
                gdk::Key::Tab => {
                    let shift_pressed = state.contains(gdk::ModifierType::SHIFT_MASK);
                    if shift_pressed {
                        if index == 0 {
                            total_buttons.saturating_sub(1)
                        } else {
                            index.saturating_sub(1)
                        }
                    } else {
                        (index + 1) % total_buttons
                    }
                }
                gdk::Key::ISO_Left_Tab => {
                    if index == 0 {
                        total_buttons.saturating_sub(1)
                    } else {
                        index.saturating_sub(1)
                    }
                }
                _ => return Propagation::Proceed,
            };
            if new_index != index {
                current_index.set(new_index);
                if let Some(button) = buttons.get(new_index) {
                    button.grab_focus();
                }
            }
            Propagation::Stop
        }
    });
    window.add_controller(controller);
}
