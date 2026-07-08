use crate::config::{ButtonConfig, Config};
use crate::ui::build_ui;
use gtk4::prelude::*;
use gtk4::Application;
use im::HashMap;
use std::env;
use std::path::PathBuf;
use std::sync::Once;
use tempfile::TempDir;

static INIT: Once = Once::new();

/// Initializes the test environment and logger (idempotent across tests).
pub fn init_env() {
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(true).try_init();
        env::set_var("FIN_SYSTEM_CONFIG", "/nonexistent/path/config.toml");
    });
}

/// Returns a dedicated temporary directory for tests using tempfile.
pub fn get_temp_dir() -> anyhow::Result<TempDir> {
    Ok(TempDir::new()?)
}

/// Builds a `Config` with the given CSS path and GTK-theme flag for UI tests.
pub fn dummy_config(css_path: Option<String>, use_gtk_theme: bool) -> Config {
    Config {
        title: "Test UI".to_string(),
        columns: 2,
        buttons: im::vector![
            ButtonConfig {
                label: "Test1".to_string(),
                command: "echo test1".to_string(),
                css_classes: None,
                widget_name: None,
            },
            ButtonConfig {
                label: "Test2".to_string(),
                command: "echo test2".to_string(),
                css_classes: None,
                widget_name: None,
            },
            ButtonConfig {
                label: "Test3".to_string(),
                command: "echo test3".to_string(),
                css_classes: None,
                widget_name: None,
            }
        ],
        use_gtk_theme,
        css_path,
        default_commands: HashMap::new(),
        layout: None,
        theme: None,
    }
}

/// Runs `build_ui` on a throwaway application, returning its `Result` over a channel.
pub fn run_build_ui(config: Config, stylesheet_path: Option<PathBuf>) -> anyhow::Result<()> {
    let (tx, rx) = std::sync::mpsc::channel();

    let app = Application::builder()
        .application_id("com.example.test.ui")
        .build();

    app.connect_activate(move |app| {
        let res = build_ui(app, &config, stylesheet_path.clone(), &config.buttons);
        let _ = tx.send(res);
        app.quit();
    });

    app.run();
    rx.recv()
        .map_err(|_| anyhow::anyhow!("No result from build_ui"))?
        .map_err(anyhow::Error::from)
}
