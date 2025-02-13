use anyhow::{anyhow, Context, Result};
use clap::{parser::ValueSource, Arg, Command};
use glib::Propagation;
use gtk4::{
    gdk, prelude::*, Application, ApplicationWindow, Button, CssProvider, EventControllerFocus,
    EventControllerKey, Grid,
};
use im::{vector, HashMap, Vector};
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
            let resolved = user_path
                .is_absolute()
                .then(|| user_path.clone())
                .unwrap_or_else(|| {
                    config_path
                        .parent()
                        .map_or(user_path.clone(), |parent| parent.join(&user_path))
                });

            // Check existence without moving
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
        })
        .unwrap_or_else(|| default_css.to_path_buf());

    let final_path = css_candidate
        .exists()
        .then(|| {
            info!("Using CSS file at '{}'.", css_candidate.display());
            css_candidate
        })
        .or_else(|| {
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
                    // Check existence without moving
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
        warn!("No valid CSS file found. Falling back to GTK built-in style.");
        None
    }))
}

#[derive(Deserialize)]
struct SystemConfig {
    #[serde(default, alias = "stylesheet")]
    css_path: Option<String>,
}

/// Loads the CSS file. If `use_gtk_theme` is true, no custom CSS is applied and the system theme remains.
/// Otherwise, loads the specified CSS file and applies it with high priority.
fn load_css(path: &Path, use_gtk_theme: bool) -> Result<()> {
    // If the system theme is desired, do not load any custom CSS.
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
            // Load an empty string to effectively apply no custom CSS.
            provider.load_from_string("");
        }
    }
    let display =
        gdk::Display::default().ok_or_else(|| anyhow!("Could not get default display"))?;
    gtk4::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION, // Use high priority for full control.
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
    #[serde(default = "default_use_gtk_theme")]
    use_gtk_theme: bool,
    /// Path to the CSS stylesheet (alias "stylesheet").
    #[serde(default, alias = "stylesheet")]
    css_path: Option<String>,
    /// Desktop environment specific button overrides.
    #[serde(default)]
    de_overrides: HashMap<String, Vector<ButtonConfig>>,
    /// Default button commands keyed by desktop environment.
    #[serde(default)]
    default_commands: HashMap<String, Vector<ButtonConfig>>,
}

#[derive(Deserialize, Debug, Clone)]
struct ButtonConfig {
    /// The label to display on the button.
    label: String,
    /// The command to execute when the button is clicked.
    command: String,
}

/// Reads and parses the configuration file.
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

/// Returns the command set appropriate for the given desktop environment.
fn get_commands_for_de(de: &str, config: &Config) -> Vector<ButtonConfig> {
    match (
        config.de_overrides.get(de).filter(|cmds| !cmds.is_empty()),
        config
            .default_commands
            .get(de)
            .filter(|cmds| !cmds.is_empty()),
        config
            .default_commands
            .get("default")
            .filter(|cmds| !cmds.is_empty()),
    ) {
        (Some(cmds), _, _) => cmds.clone(),
        (None, Some(cmds), _) => cmds.clone(),
        (None, None, Some(cmds)) => cmds.clone(),
        _ => config.buttons.clone(),
    }
}

//
// 3. UI and Navigation Functions
//

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
    match key {
        // Vertical navigation using modular arithmetic
        gdk::Key::Up | gdk::Key::KP_Up => current
            .checked_sub(columns)
            .unwrap_or_else(|| {
                // Calculate last row starting position
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

        // Horizontal navigation using modular arithmetic
        gdk::Key::Left | gdk::Key::KP_Left => (current + total - 1) % total,
        gdk::Key::Right | gdk::Key::KP_Right => (current + 1) % total,

        _ => current,
    }
}

/// Calculates the new index for Tab navigation using modular arithmetic
fn calculate_new_index_for_tab(index: usize, total: usize, forward: bool) -> usize {
    match forward {
        true => (index + 1) % total,
        false => (index + total - 1) % total,
    }
}

/// Converts a vector of button configurations into a vector of GTK buttons.
fn create_buttons(app: &Application, btn_configs: &Vector<ButtonConfig>) -> Vector<Button> {
    btn_configs
        .iter()
        .map(|btn_cfg| create_action_button(app, &btn_cfg.label, &btn_cfg.command))
        .collect::<Vector<_>>()
}

/// Creates a GTK button that executes a command when clicked.
fn create_action_button(app: &Application, label: &str, command: &str) -> Button {
    let button = Button::with_label(label);
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

/// Displays an error notification via logging.
fn show_error_dialog(_app: &Application, message: &str) {
    error!("Error Notification: {}", message);
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

/// Sets up key event handlers for cyclic navigation of the buttons.
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

/// Builds the user interface.
/// The `stylesheet_path` parameter is an Option<PathBuf>. If it is None,
/// the UI code forces use of the built‑in style.
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

    let window = ApplicationWindow::builder()
        .application(app)
        .title(&config.title)
        .default_width(600)
        .default_height(400)
        .build();
    window.set_decorated(false);
    window.set_transient_for(None::<&ApplicationWindow>);
    window.set_resizable(false);
    window.set_tooltip_text(Some("Finë logout manager window"));

    // If use_gtk_theme is true, we let the system theme handle styling.
    // Otherwise, we load our custom CSS (if provided).
    match stylesheet_path {
        Some(ref css_path) => load_css(css_path, false)?,
        None => load_css(&PathBuf::new(), true)?,
    }

    let grid = create_grid();
    grid.set_tooltip_text(Some("Button grid"));
    let layout = calculate_layout(buttons.len()).map_err(|e| anyhow!(e))?;
    let all_buttons = create_buttons(app, buttons);
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

/// Creates a new GTK grid container.
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

//
// 4. Main Entry Point
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
    let commands = get_commands_for_de(&de, &config);
    let default_css = PathBuf::from("/usr/share/fin/style.css");

    // Use the updated select_css_path which returns Option<PathBuf>
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
    use im::hashmap;
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
    fn load_config_valid_file() {
        init_env();
        let path = PathBuf::from("assets/config.toml");
        let config = load_config(&path).expect("Failed to load valid config");
        assert_eq!(config.title, "Finë");
        assert_eq!(config.columns, 2);
        assert_eq!(config.buttons.len(), 0);
    }

    #[test]
    fn load_config_invalid_file() {
        init_env();
        let path = PathBuf::from("tests/fixtures/invalid_config.toml");
        let result = load_config(&path);
        assert!(result.is_err());
    }

    #[test]
    fn load_config_nonexistent_file() {
        init_env();
        let path = PathBuf::from("tests/fixtures/nonexistent_config.toml");
        let result = load_config(&path);
        assert!(result.is_err());
    }

    #[test]
    fn calculate_layout_valid_buttons() {
        init_env();
        let layout = calculate_layout(4).expect("Failed to calculate layout");
        assert_eq!(layout.len(), 2);
        assert_eq!(layout[0].len(), 2);
        assert_eq!(layout[1].len(), 2);
    }

    #[test]
    fn calculate_layout_invalid_buttons() {
        init_env();
        let result = calculate_layout(7);
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
    fn get_commands_for_de_with_override() {
        init_env();
        let config = Config {
            title: "Test".to_string(),
            columns: 1,
            buttons: vector![],
            use_gtk_theme: false,
            css_path: None,
            de_overrides: hashmap! {
                "test_de".to_string() => vector![ButtonConfig {
                    label: "Override".to_string(),
                    command: "echo override".to_string()
                }]
            },
            default_commands: hashmap! {},
        };
        let commands = get_commands_for_de("test_de", &config);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label, "Override");
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
            de_overrides: hashmap! {},
            default_commands: hashmap! {
                "default".to_string() => vector![ButtonConfig {
                    label: "Default".to_string(),
                    command: "echo default".to_string()
                }]
            },
        };
        let commands = get_commands_for_de("unknown_de", &config);
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].label, "Default");
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
    fn test_select_css_path_missing_user_css_fallback_to_default() -> Result<()> {
        init_env();
        let tmp_dir = get_temp_dir()?;
        let config_path = tmp_dir.join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;

        let user_css = Some("nonexistent.css".to_string());

        let default_css = tmp_dir.join("default.css");
        fs::write(&default_css, "button { background-color: green; }")?;

        let result = select_css_path(user_css, &config_path, &default_css, false)?;
        assert_eq!(result, Some(default_css.clone()));

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

    /// Create a minimal dummy configuration.
    fn dummy_config(css_path: Option<String>, use_gtk_theme: bool) -> Config {
        Config {
            title: "Test UI".to_string(),
            columns: 1,
            buttons: vector![ButtonConfig {
                label: "Test".to_string(),
                command: "echo test".to_string(),
            }],
            use_gtk_theme,
            css_path,
            de_overrides: HashMap::new(),
            default_commands: HashMap::new(),
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
