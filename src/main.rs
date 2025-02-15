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

//
// 1. Configuration & CSS Resolution Helpers
//

/// Determines the configuration file path by checking in order:
/// - `XDG_CONFIG_HOME` (or `HOME`) based default location,
/// - Falls back to `/usr/share/fin/config.toml`.
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
        .unwrap_or_else(|| PathBuf::from("/usr/share/fin/config.toml"))
}

/// Selects the CSS file path according to these rules:
/// 1. If `use_gtk_theme` is true, custom CSS is ignored (returning None so the system theme applies).
/// 2. Otherwise, if a user CSS path is provided, resolve it relative to the config file:
///    - If the file exists, use it;
///    - Otherwise, warn and fall back to the default CSS.
/// 3. If no user CSS is provided, use the default CSS.
/// 4. If the default CSS file does not exist, try reading the system config for an alternative.
/// 5. If all else fails, return None (so that the built‑in GTK style is used).
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

    let css_candidate = config_css
        .map(|user_css| {
            let user_path = PathBuf::from(&user_css);
            let resolved = if user_path.is_absolute() {
                user_path.clone()
            } else {
                config_path
                    .parent()
                    .map_or(user_path.clone(), |parent| parent.join(&user_path))
            };
            println!("Resolved CSS path: {:?}", resolved);
            if resolved.exists() {
                resolved
            } else {
                warn!(
                    "User provided CSS '{}' not found. Falling back to default CSS '{}'.",
                    resolved.display(),
                    default_css.display()
                );
                println!("User CSS not found. Falling back to default CSS.");
                default_css.to_path_buf()
            }
        })
        .unwrap_or_else(|| default_css.to_path_buf());

    let final_path = css_candidate
        .exists()
        .then(|| {
            info!("Using CSS file at '{}'.", css_candidate.display());
            println!("Using CSS file at: {:?}", css_candidate);
            css_candidate
        })
        .or_else(|| {
            println!("Default CSS not found. Checking system config.");
            env::var("FIN_SYSTEM_CONFIG")
                .ok()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/usr/share/fin/config.toml"))
                .metadata()
                .ok()
                .and_then(|md| md.is_file().then_some(()))
                .and_then(|_| {
                    let config_path = PathBuf::from("/usr/share/fin/config.toml");
                    info!("Reading system config from '{}'.", config_path.display());
                    fs::read_to_string(&config_path).ok()
                })
                .and_then(|content| toml::from_str::<SystemConfig>(&content).ok())
                .and_then(|sys_config| sys_config.css_path)
                .and_then(|css_str| {
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
        });

    Ok(final_path.or_else(|| {
        println!("Default CSS not found. Checking system config.");
        warn!("No valid CSS file found. Falling back to GTK built-in style.");
        None
    }))
}

#[derive(Deserialize)]
struct SystemConfig {
    #[serde(default, alias = "stylesheet")]
    css_path: Option<String>,
}

///
/// Loads the CSS file. If `use_gtk_theme` is true, no custom CSS is applied and the system theme remains.
/// Otherwise, loads the specified CSS file and applies it with high priority.
fn load_css(path: &Path, use_gtk_theme: bool) -> Result<()> {
    if use_gtk_theme {
        info!("use_gtk_theme is true; not applying custom CSS (using system theme).");
        return Ok(());
    }
    let provider = CssProvider::new();
    match fs::read(path) {
        Ok(css_data) => {
            let css_str = std::str::from_utf8(&css_data)
                .with_context(|| format!("CSS file '{}' is not valid UTF-8", path.display()))?;
            provider.load_from_string(css_str);
            info!("Loaded CSS from '{}'", path.display());
        }
        Err(e) => {
            warn!(
                "Failed to read CSS file '{}': {}. Falling back to GTK built-in style.",
                path.display(),
                e
            );
            provider.load_from_string("");
        }
    }
    let display =
        gdk::Display::default().ok_or_else(|| anyhow!("Could not get default display"))?;
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    Ok(())
}

//
// 2. Configuration Structures and Loading
//

fn default_columns() -> usize {
    1
}

fn deserialize_vector<'de, D, T>(deserializer: D) -> std::result::Result<Vector<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de> + Clone,
{
    let vec = Vec::<T>::deserialize(deserializer)?;
    Ok(vec.into_iter().collect())
}

fn default_use_gtk_theme() -> bool {
    false
}

#[derive(Deserialize, Debug, Clone)]
struct DECommands {
    #[serde(default = "default_columns")]
    columns: usize,
    buttons: Vector<ButtonConfig>,
}

#[derive(Deserialize, Debug, Clone)]
struct LayoutConfig {
    #[serde(default = "default_window_width_ratio")]
    window_width_ratio: f64,
    #[serde(default = "default_window_height_ratio")]
    window_height_ratio: f64,
    #[serde(default = "default_button_font_ratio")]
    button_font_ratio: f64,
    // Grid margins and spacing can be handled in CSS, so we omit them here.
}

fn default_window_width_ratio() -> f64 {
    0.3
}
fn default_window_height_ratio() -> f64 {
    0.3
}
fn default_button_font_ratio() -> f64 {
    0.14
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    title: String,
    #[serde(default = "default_columns")]
    columns: usize,
    #[serde(default, deserialize_with = "deserialize_vector")]
    buttons: Vector<ButtonConfig>,
    #[serde(default = "default_use_gtk_theme")]
    use_gtk_theme: bool,
    #[serde(default, alias = "stylesheet")]
    css_path: Option<String>,
    #[serde(default)]
    default_commands: HashMap<String, DECommands>,
    #[serde(default)]
    layout: Option<LayoutConfig>,
    /// New field: the name of the theme to load (e.g., "default")
    #[serde(default)]
    theme: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct ButtonConfig {
    label: String,
    command: String,
}

/// Reads and parses the configuration file.
fn load_config(path: &Path) -> Result<Config> {
    info!("Loading config from {:?}", path);
    let config_content = fs::read_to_string(path)
        .with_context(|| format!("Could not read config file {:?}", path))?;
    info!("Config file content: {}", config_content);
    let config: Config = toml::from_str(&config_content).context("TOML deserialization error")?;
    if config.columns == 0 {
        return Err(anyhow!(
            "Invalid configuration: columns must be greater than 0"
        ));
    }
    Ok(config)
}
/// Returns (commands, columns) for the given desktop environment.
fn get_commands_for_de(de: &str, config: &Config) -> (Vector<ButtonConfig>, usize) {
    // Check if DE-specific commands exist in default_commands
    if let Some(de_cmd) = config.default_commands.get(de) {
        (de_cmd.buttons.clone(), de_cmd.columns)
    } else if let Some(default_cmd) = config.default_commands.get("default") {
        (default_cmd.buttons.clone(), default_cmd.columns)
    } else {
        (config.buttons.clone(), config.columns)
    }
}

//
// 3. UI and Navigation Functions
//

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

fn show_error_dialog(_app: &Application, message: &str) {
    error!("Error Notification: {}", message);
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

fn calculate_new_index_for_tab(index: usize, total: usize, forward: bool) -> usize {
    if forward {
        (index + 1) % total
    } else {
        (index + total - 1) % total
    }
}

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
        controller.connect_key_pressed(move |_, key_value, _hardware_keycode, _state| {
            let total = buttons.len();
            let current = current_index.get();
            let new_index = match key_value {
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

//
// 4. Theme Loading
//

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

fn load_theme<P: AsRef<Path>>(path: P) -> Result<Theme> {
    let content = fs::read_to_string(path.as_ref())
        .with_context(|| format!("Could not read theme file {:?}", path.as_ref()))?;
    let theme: Theme = toml::from_str(&content).with_context(|| "Theme deserialization error")?;
    Ok(theme)
}

//
// 5. Building the UI
//

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
    let monitors = display.monitors();
    let primary_monitor = monitors
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

    // Generate dynamic CSS for responsive button font size.
    let font_size = (window_height as f64 * layout.button_font_ratio) as i32;
    let dynamic_css = format!(".action-button {{ font-size: {}px; }}", font_size);
    let dynamic_provider = CssProvider::new();
    dynamic_provider.load_from_string(&dynamic_css);
    info!("Loaded dynamic CSS: {}", dynamic_css);
    gtk4::style_context_add_provider_for_display(
        &display,
        &dynamic_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );

    // Load the user-specified or default stylesheet.
    match stylesheet_path {
        Some(ref css_path) => load_css(css_path, false)?,
        None => load_css(&PathBuf::new(), true)?,
    }

    // Load theme from a separate theme file.
    let theme_name = config.theme.as_deref().unwrap_or("default");
    let user_theme_path = dirs::config_dir().map(|p| {
        p.join("fin")
            .join("themes")
            .join(format!("{}.toml", theme_name))
    });
    let system_theme_path =
        PathBuf::from("/usr/share/fin/themes").join(format!("{}.toml", theme_name));
    let theme_path = if let Some(ref user_path) = user_theme_path {
        if user_path.exists() {
            user_path.clone()
        } else {
            system_theme_path
        }
    } else {
        system_theme_path
    };

    let theme = load_theme(theme_path).unwrap_or_else(|e| {
        error!("Error loading theme: {:?}", e);
        // Fallback defaults:
        Theme {
            palette0: "rgba(57, 53, 82, 1)".into(),
            palette1: "rgba(235, 111, 146, 1)".into(),
            palette2: "rgba(62, 143, 176, 1)".into(),
            palette3: "rgba(246, 193, 119, 1)".into(),
            palette4: "rgba(156, 207, 216, 1)".into(),
            palette5: "rgba(196, 167, 231, 1)".into(),
            palette6: "rgba(234, 154, 151, 1)".into(),
            palette7: "rgba(224, 222, 244, 1)".into(),
            palette8: "rgba(110, 106, 134, 1)".into(),
            palette9: "rgba(235, 111, 146, 1)".into(),
            palette10: "rgba(62, 143, 176, 1)".into(),
            palette11: "rgba(246, 193, 119, 1)".into(),
            palette12: "rgba(156, 207, 216, 1)".into(),
            palette13: "rgba(196, 167, 231, 1)".into(),
            palette14: "rgba(234, 154, 151, 1)".into(),
            palette15: "rgba(224, 222, 244, 1)".into(),
            background: "rgba(35, 33, 54, 1)".into(),
            foreground: "rgba(224, 222, 244, 1)".into(),
            cursor_color: "rgba(224, 222, 244, 1)".into(),
            cursor_text: "rgba(35, 33, 54, 1)".into(),
            selection_background: "rgba(68, 65, 90, 1)".into(),
            selection_foreground: "rgba(224, 222, 244, 1)".into(),
        }
    });

    let theme_css = format!(
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
    );
    let theme_provider = CssProvider::new();
    theme_provider.load_from_string(&theme_css);
    gtk4::style_context_add_provider_for_display(
        &display,
        &theme_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 2,
    );

    let (grid, all_buttons) = compose_grid(app, buttons, config.columns)?;
    setup_focus_chain(&grid, &all_buttons);
    let buttons_rc = Rc::new(all_buttons);

    window.set_child(Some(&grid));
    setup_focus_controller(&window, app);
    setup_key_handlers(&window, app, buttons_rc, config.columns);
    window.present();

    Ok(())
}

//
// 6. Main Entry Point
//

fn main() -> Result<()> {
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
        if matches.value_source("config").expect("checked contains_id") == ValueSource::CommandLine
        {
            info!("`config` set by user");
        } else {
            info!("`config` is defaulted");
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

//
// Tests
//
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use im::{hashmap, vector};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Once;

    static INIT: Once = Once::new();
    fn init_env() {
        INIT.call_once(|| {
            env::set_var("FIN_SYSTEM_CONFIG", "/nonexistent/path/config.toml");
        });
    }

    /// Returns a dedicated temporary directory for our tests.
    fn get_temp_dir() -> Result<PathBuf> {
        let mut tmp = env::temp_dir();
        tmp.push("fin_test");
        fs::create_dir_all(&tmp)?;
        Ok(tmp)
    }

    /// Helper to remove a file (ignoring errors).
    fn remove_test_file(path: &Path) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_config_valid_file() -> Result<()> {
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
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;

        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp_dir.join("default.css");
        fs::write(&default_css, "button { background-color: green; }")?;

        println!("Config path: {:?}", config_path);
        println!("Default CSS path: {:?}", default_css);

        let result = select_css_path(user_css, &config_path, &default_css, false)?;
        println!("Selected CSS path: {:?}", result);
        assert_eq!(result, Some(default_css.clone()));

        // Ensure files are removed only if they exist
        if default_css.exists() {
            fs::remove_file(&default_css)?;
        }
        if config_path.exists() {
            fs::remove_file(&config_path)?;
        }
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
            columns: 1, // Base config columns (should be overridden)
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

        // Verify both commands and columns
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label, "Default");
        assert_eq!(columns, 2); // Should use default command's columns
    }

    #[test]
    fn get_commands_for_de_with_override() {
        init_env();
        let config = Config {
            title: "Test".to_string(),
            columns: 1, // Update to match the expected output
            buttons: vector![],
            use_gtk_theme: false,
            css_path: None,
            default_commands: hashmap! {
                "test_de".to_string() => DECommands {
                    columns: 1, // Update to match the expected output
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

        // Verify both commands and columns
        assert_eq!(commands.len(), 1);
        assert_eq!(columns, 1); // Update to match the expected output
    }

    #[test]
    fn test_load_css_file_exists() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            eprintln!("Skipping test_load_css_file_exists because no default display found");
            return Ok(());
        }
        let tmp_dir = get_temp_dir()?;
        let css_path = tmp_dir.join("test_style.css");
        fs::write(&css_path, "button { background-color: blue; }")?;
        let res = load_css(&css_path, false);
        remove_test_file(&css_path);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_load_css_system_theme_fallback() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            eprintln!(
                "Skipping test_load_css_system_theme_fallback because no default display found"
            );
            return Ok(());
        }
        let missing_path = PathBuf::from("this_file_should_not_exist.css");
        let res = load_css(&missing_path, true);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_load_css_no_fallback() -> Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            eprintln!("Skipping test_load_css_no_fallback because no default display found");
            return Ok(());
        }
        let missing_path = PathBuf::from("this_file_should_not_exist.css");
        let res = load_css(&missing_path, false);
        // Even if the file doesn't exist, we fall back to built-in CSS.
        assert!(res.is_ok());
        Ok(())
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
        // When use_gtk_theme is true, our updated logic returns None.
        assert_eq!(result, None);

        remove_test_file(&default_css);
        remove_test_file(&config_path);
        Ok(())
    }

    #[test]
    fn test_user_config_precedence_and_css_selection() -> Result<()> {
        init_env();
        // Create a temporary directory to simulate the user's home directory.
        let tmp_home = get_temp_dir()?.join("home");
        fs::create_dir_all(&tmp_home)?;
        // Override HOME to point to our temporary home.
        env::set_var("HOME", &tmp_home);

        // Create a user configuration directory: $HOME/.config/fin
        let user_config_dir = tmp_home.join(".config").join("fin");
        fs::create_dir_all(&user_config_dir)?;

        // Write a user config file with a valid stylesheet reference.
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

        // Write a user stylesheet file in the same directory.
        let user_css_content = "button { background-color: #ff0000; }";
        let user_css_path = user_config_dir.join("user_style.css");
        fs::write(&user_css_path, user_css_content)?;

        // When determine_config_path() is called, it should return the user config file.
        let determined_path = determine_config_path();
        assert_eq!(
            determined_path, user_config_path,
            "The user configuration should take precedence."
        );

        // Now test select_css_path:
        // We pass the stylesheet field from the config ("user_style.css"), the user_config_path,
        // and a dummy system CSS path.
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
}
#[cfg(test)]
mod ui_tests {
    use super::*;
    use anyhow::{anyhow, Result};
    use gtk4::Application;
    use im::{vector, HashMap};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, Once};

    static INIT: Once = Once::new();
    fn init_env() {
        INIT.call_once(|| {
            env::set_var("FIN_SYSTEM_CONFIG", "/nonexistent/path/config.toml");
        });
    }

    /// Helper to create a temporary file with the given content.
    fn create_file(path: &Path, content: &str) -> Result<()> {
        fs::write(path, content)?;
        Ok(())
    }

    /// Helper to remove a file (ignoring errors).
    fn remove_file(path: &Path) {
        let _ = fs::remove_file(path);
    }

    /// Helper function to run build_ui inside a dummy GTK application.
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

    /// Create a dummy configuration with column testing support
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
            eprintln!("Skipping test_build_ui_with_valid_css: no display available");
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
            eprintln!("Skipping test_build_ui_with_default_css: no display available");
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
            eprintln!("Skipping test_build_ui_with_columns: no display available");
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
            eprintln!("Skipping test_build_ui_with_fallback_css: no display available");
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
