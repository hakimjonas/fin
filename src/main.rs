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
use im::HashMap;
use serde::Deserialize;
use std::cell::Cell;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

fn deservvialize_vector<'de, D, T>(deserializer: D) -> std::result::Result<Vector<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Clone,
{
    let vec = Vec::<T>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    title: String,
    columns: usize,
    #[serde(default, deserialize_with = "deserialize_vector")]
    buttons: Vector<ButtonConfig>,
    use_system_theme: bool,
    #[serde(default)]
    de_overrides: HashMap<String, Vector<ButtonConfig>>,
    #[serde(default)]
    default_commands: HashMap<String, Vector<ButtonConfig>>,
}

#[derive(Deserialize, Debug, Clone)]
struct ButtonConfig {
    label: String,
    command: String,
}

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

fn main() -> Result<()> {
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
            println!("`config` set by user");
        } else {
            println!("`config` is defaulted");
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
            eprintln!("Error building UI: {:?}", e);
            std::process::exit(1);
        }
    });

    app.run();
    Ok(())
}

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

    load_css(stylesheet_path, config.use_system_theme)?;

    let grid = create_grid();
    attach_buttons_to_grid(&grid, buttons, config.columns, app)?;

    window.set_child(Some(&grid));
    setup_focus_controller(&window, app);
    setup_key_handlers(&window, app, collect_buttons(&grid), config.columns);
    window.present();
    Ok(())
}
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

fn attach_buttons_to_grid(
    grid: &Grid,
    buttons: &Vector<ButtonConfig>,
    columns: usize,
    app: &Application,
) -> Result<()> {
    if columns == 0 {
        return Err(anyhow!(
            "Invalid configuration: columns must be greater than 0"
        ));
    }

    let rows = (buttons.len() + columns - 1) / columns;
    let layout: Vector<Vector<usize>> = (0..rows)
        .map(|row| {
            (0..columns)
                .filter_map(|col| {
                    let index = row * columns + col;
                    if index < buttons.len() {
                        Some(index)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .collect();

    let button_widgets: Vector<Button> = layout
        .iter()
        .enumerate()
        .flat_map(|(row, cols)| {
            cols.iter().enumerate().map(move |(col, &index)| {
                let button_config = &buttons[index];
                let button =
                    create_action_button(app, &button_config.label, &button_config.command);
                println!(
                    "Attaching button '{}' at row {}, col {}",
                    button_config.label, row, col
                );
                grid.attach(&button, col as i32, row as i32, 1, 1);
                button
            })
        })
        .collect();

    setup_focus_chain(grid, &button_widgets);
    Ok(())
}
fn setup_focus_chain(grid: &Grid, buttons: &Vector<Button>) {
    if let Some(first_button) = buttons.get(0) {
        grid.set_focus_child(Some(first_button));
    }
}

fn setup_focus_controller(window: &ApplicationWindow, app: &Application) {
    let app_clone = app.clone();
    let focus_controller = EventControllerFocus::new();
    focus_controller.connect_leave(move |_| {
        app_clone.quit();
    });
    window.add_controller(focus_controller);
}

fn collect_buttons(grid: &Grid) -> Vector<Button> {
    std::iter::successors(grid.first_child(), |child| child.next_sibling())
        .filter_map(|w| w.downcast::<Button>().ok())
        .collect()
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
            if let Err(e) = ProcessCommand::new("sh")
                .arg("-c")
                .arg(&command_string)
                .spawn()
            {
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
            handle_key_press(
                app.clone(),
                buttons.clone(),
                current_index.clone(),
                columns,
                keyval,
                state,
            )
        }
    });
    window.add_controller(controller);
}

fn handle_key_press(
    app: Application,
    buttons: Vector<Button>,
    current_index: std::rc::Rc<Cell<usize>>,
    columns: usize,
    keyval: gdk::Key,
    state: gdk::ModifierType,
) -> Propagation {
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
        gdk::Key::Up | gdk::Key::KP_Up => {
            calculate_new_index(index, columns, |i, c| i.saturating_sub(c))
        }
        gdk::Key::Down | gdk::Key::KP_Down => calculate_new_index(index, columns, |i, c| {
            (i + c).min(total_buttons.saturating_sub(1))
        }),
        gdk::Key::Left | gdk::Key::KP_Left => index.saturating_sub(1),
        gdk::Key::Right | gdk::Key::KP_Right => (index + 1).min(total_buttons.saturating_sub(1)),
        gdk::Key::Tab => handle_tab_key(index, total_buttons, state),
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

fn handle_tab_key(index: usize, total_buttons: usize, state: gdk::ModifierType) -> usize {
    if state.contains(gdk::ModifierType::SHIFT_MASK) {
        if index == 0 {
            total_buttons.saturating_sub(1)
        } else {
            index.saturating_sub(1)
        }
    } else {
        (index + 1) % total_buttons
    }
}

fn calculate_new_index<F>(index: usize, columns: usize, f: F) -> usize
where
    F: Fn(usize, usize) -> usize,
{
    if columns > 0 {
        f(index, columns)
    } else {
        index
    }
}
