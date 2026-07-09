use crate::config::{ButtonConfig, ButtonConfigs, Config, LayoutConfig};
use crate::error::AppError;
use crate::nav::FocusState;
use crate::theme::get_theme_css;

use glib::Propagation;
use gtk4::gdk;
use gtk4::{
    gdk::Monitor, prelude::*, AlertDialog, Application, ApplicationWindow, Button, CssProvider,
    EventControllerFocus, EventControllerKey, Grid,
};
use im::Vector;
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;
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

/// The widget-side collection of created buttons.
type Buttons = Vector<Button>;

// ---------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------

/// Executes the given shell command via `sh -c`.
fn execute_command(command: &str) -> Result<(), AppError> {
    if command.is_empty() {
        return Ok(());
    }
    ProcessCommand::new("sh")
        .arg("-c")
        .arg(command)
        .spawn()
        .map_err(|e| {
            AppError::Command(format!("Failed to execute command '{}': {}", command, e))
        })?;
    Ok(())
}

/// Creates an action button bound to `execute` for its click handler.
fn create_action_button<F>(app: &Application, cfg: &ButtonConfig, execute: F) -> Button
where
    F: Fn(&str) -> Result<(), AppError> + 'static,
{
    let button = Button::with_label(&cfg.label);
    button.add_css_class("action-button");

    if let Some(classes) = &cfg.css_classes {
        for class in classes {
            button.add_css_class(class);
        }
    }

    if let Some(name) = &cfg.widget_name {
        button.set_widget_name(name);
    }

    button.set_tooltip_text(Some(&format!("Executes command: {}", cfg.command)));
    let command_string = cfg.command.clone();
    let app_clone = app.clone();
    button.connect_clicked(move |_| {
        if let Err(e) = execute(&command_string) {
            show_error_dialog(&app_clone, &e.to_string());
        }
        app_clone.quit();
    });
    button
}

/// Displays an error message using a GTK modal dialog.
fn show_error_dialog(_app: &Application, message: &str) {
    let dialog = AlertDialog::builder()
        .modal(true)
        .message("Error")
        .detail(message)
        .buttons(&["Ok"][..])
        .build();

    dialog.show(None::<&ApplicationWindow>);
}

/// Assigns the first button as the focus child in the grid.
fn setup_focus_chain(grid: &Grid, buttons: &Buttons) {
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

/// Sets up key handlers for the window to handle navigation and button activation.
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

    let focus_state = Rc::new(Cell::new(FocusState::new()));
    let controller = EventControllerKey::new();

    let app_clone = app.clone();
    let buttons_ref = Rc::clone(&buttons);
    let focus_state_clone = Rc::clone(&focus_state);

    controller.connect_key_pressed(move |_, key_value, _hardware_keycode, _state| {
        match key_value {
            gdk::Key::Escape => {
                app_clone.quit();
                return Propagation::Stop;
            }
            gdk::Key::Return => {
                if let Some(button) = buttons_ref.get(focus_state_clone.get().current()) {
                    button.emit_clicked();
                }
                return Propagation::Stop;
            }
            _ => {}
        }

        let total = buttons_ref.len();
        let new_state = focus_state_clone
            .get()
            .transition(key_value, total, columns);
        if new_state.current() != focus_state_clone.get().current() {
            focus_state_clone.set(new_state);
            if let Some(button) = buttons_ref.get(new_state.current()) {
                button.grab_focus();
            }
        }
        Propagation::Stop
    });

    window.add_controller(controller);
}

/// Composes a grid layout and attaches action buttons to it.
fn compose_grid<F>(
    app: &Application,
    buttons: &ButtonConfigs,
    columns: usize,
    execute: F,
) -> Result<(Grid, Buttons), AppError>
where
    F: Fn(&str) -> Result<(), AppError> + Copy + 'static,
{
    let grid = create_grid();
    let all_buttons: Buttons = buttons
        .iter()
        .enumerate()
        .map(|(index, cfg)| {
            let button = create_action_button(app, cfg, execute);
            let row = index / columns;
            let col = index % columns;
            grid.attach(&button, col as i32, row as i32, 1, 1);
            log::info!(
                "Attached button '{}' at row {}, col {}",
                cfg.label,
                row,
                col
            );
            button
        })
        .collect();
    Ok((grid, all_buttons))
}

/// Creates a new `Grid` widget with default spacing and margins.
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

/// Detects the current desktop environment using the `XDG_CURRENT_DESKTOP` or
/// `DESKTOP_SESSION` environment variables.
#[must_use]
pub fn detect_desktop_environment() -> String {
    std::env::var("XDG_CURRENT_DESKTOP")
        .or_else(|_| std::env::var("DESKTOP_SESSION"))
        .unwrap_or_default()
        .to_lowercase()
        .split(':')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Loads combined CSS into a single `CssProvider`.
#[must_use]
pub fn load_combined_css(dynamic_css: &str, user_css: &str, theme_css: &str) -> CssProvider {
    let combined_css = format!("{}\n{}\n{}", dynamic_css, user_css, theme_css);
    let provider = CssProvider::new();
    provider.load_from_string(&combined_css);
    provider
}

/// Pure: compute window dimensions from monitor geometry + layout config.
#[must_use]
pub fn compute_window_size(geom: &gdk::Rectangle, layout: &LayoutConfig) -> (i32, i32) {
    let w = (geom.width() as f64 * layout.window_width_ratio) as i32;
    let h = (geom.height() as f64 * layout.window_height_ratio) as i32;
    (w, h)
}

/// Pure: generate dynamic CSS rule for button font size.
#[must_use]
pub fn dynamic_button_css(window_height: i32, font_ratio: f64) -> String {
    let size = (window_height as f64 * font_ratio) as i32;
    format!(".action-button {{ font-size: {}px; }}", size)
}

/// Builds the application UI by combining dynamic CSS, user CSS, and theme CSS.
pub fn build_ui(
    app: &Application,
    config: &Config,
    stylesheet_path: Option<PathBuf>,
    buttons: &ButtonConfigs,
) -> Result<(), AppError> {
    if config.columns == 0 {
        return Err(AppError::Config(crate::error::ConfigError::Validation(
            "columns must be greater than 0".into(),
        )));
    }
    if buttons.is_empty() {
        return Err(AppError::Ui(
            "No buttons to display; please check your configuration".into(),
        ));
    }

    let display = gdk::Display::default()
        .ok_or_else(|| AppError::Ui("Could not get default display".into()))?;
    let primary_monitor = display
        .monitors()
        .item(0)
        .and_then(|obj| obj.downcast::<Monitor>().ok())
        .ok_or_else(|| AppError::Ui("Could not get primary monitor".into()))?;
    let geom = primary_monitor.geometry();

    let layout = config.layout.clone().unwrap_or(LayoutConfig {
        window_width_ratio: DEFAULT_WINDOW_WIDTH_RATIO,
        window_height_ratio: DEFAULT_WINDOW_HEIGHT_RATIO,
        button_font_ratio: DEFAULT_BUTTON_FONT_RATIO,
    });

    let (window_width, window_height) = compute_window_size(&geom, &layout);

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

    let dynamic_css = dynamic_button_css(window_height, layout.button_font_ratio);

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

    let (grid, all_buttons) = compose_grid(app, buttons, config.columns, execute_command)?;
    let buttons_rc = Rc::new(all_buttons);

    setup_focus_chain(&grid, &buttons_rc);
    window.set_child(Some(&grid));
    setup_focus_controller(&window, app);
    setup_key_handlers(&window, app, Rc::clone(&buttons_rc), config.columns);
    window.present();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::css_candidate;
    use crate::css::select_css_path;
    use crate::test_utils::{dummy_config, get_temp_dir, init_env, run_build_ui};
    use std::fs;

    #[test]
    fn test_build_ui_with_valid_css() -> anyhow::Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            log::info!("Skipping test_build_ui_with_valid_css: no display available");
            return Ok(());
        }
        let tmp = get_temp_dir()?;
        let valid_css = tmp.path().join("valid_test.css");
        fs::write(&valid_css, "button { background-color: purple; }")?;
        let config = dummy_config(Some(valid_css.to_string_lossy().to_string()), false);
        let config_path = tmp.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let default_css = tmp.path().join("default.css");
        fs::write(&default_css, "button { background-color: blue; }")?;
        let stylesheet_path = select_css_path(
            css_candidate(config.css_path.as_deref(), config_path.parent().unwrap()),
            &default_css,
            config.use_gtk_theme,
        );
        assert_eq!(stylesheet_path, Some(valid_css.clone()));
        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_build_ui_with_default_css() -> anyhow::Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            log::info!("Skipping test_build_ui_with_default_css: no display available");
            return Ok(());
        }
        let tmp = get_temp_dir()?;
        let config_path = tmp.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let config = dummy_config(Some("nonexistent.css".to_string()), false);
        let default_css = tmp.path().join("default.css");
        fs::write(&default_css, "button { background-color: green; }")?;
        let stylesheet_path = select_css_path(
            css_candidate(config.css_path.as_deref(), config_path.parent().unwrap()),
            &default_css,
            config.use_gtk_theme,
        );
        assert_eq!(stylesheet_path, Some(default_css.clone()));
        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_build_ui_with_columns() -> anyhow::Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            log::info!("Skipping test_build_ui_with_columns: no display available");
            return Ok(());
        }
        let config = Config {
            title: "Column Test".to_string(),
            columns: 3,
            buttons: im::vector![
                ButtonConfig {
                    label: "1".into(),
                    command: "".into(),
                    css_classes: None,
                    widget_name: None,
                },
                ButtonConfig {
                    label: "2".into(),
                    command: "".into(),
                    css_classes: None,
                    widget_name: None,
                },
                ButtonConfig {
                    label: "3".into(),
                    command: "".into(),
                    css_classes: None,
                    widget_name: None,
                },
                ButtonConfig {
                    label: "4".into(),
                    command: "".into(),
                    css_classes: None,
                    widget_name: None,
                },
                ButtonConfig {
                    label: "5".into(),
                    command: "".into(),
                    css_classes: None,
                    widget_name: None,
                },
            ],
            use_gtk_theme: false,
            css_path: None,
            default_commands: im::HashMap::new(),
            layout: None,
            theme: None,
        };
        let res = run_build_ui(config, None);
        assert!(res.is_ok());
        Ok(())
    }

    #[test]
    fn test_build_ui_with_fallback_css() -> anyhow::Result<()> {
        init_env();
        if gdk::Display::default().is_none() {
            log::info!("Skipping test_build_ui_with_fallback_css: no display available");
            return Ok(());
        }
        let tmp = get_temp_dir()?;
        let config_path = tmp.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config")?;
        let config = dummy_config(None, false);
        let default_css = tmp.path().join("nonexistent_default.css");
        let stylesheet_path = select_css_path(
            css_candidate(config.css_path.as_deref(), config_path.parent().unwrap()),
            &default_css,
            config.use_gtk_theme,
        );
        assert_eq!(stylesheet_path, None);
        let res = run_build_ui(config.clone(), stylesheet_path);
        assert!(res.is_ok());
        Ok(())
    }
}
