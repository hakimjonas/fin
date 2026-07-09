use anyhow::{Context, Result};
use clap::{Arg, Command};
use gtk4::prelude::*;
use log::error;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    env_logger::init();

    // Default to the OpenGL (GL) renderer to avoid the noisy Vulkan/radv driver
    // warnings GTK emits when it auto-selects the Vulkan backend, while keeping
    // GPU compositing for correct CSS/transparency. Honors an explicit user
    // override via the GSK_RENDERER environment variable.
    if std::env::var_os("GSK_RENDERER").is_none() {
        std::env::set_var("GSK_RENDERER", "gl");
    }

    let matches = Command::new("fin")
        .version(env!("CARGO_PKG_VERSION"))
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

    let config_path = matches
        .get_one::<String>("config")
        .map(PathBuf::from)
        .unwrap_or_else(fin::config::determine_config_path);

    let config = fin::config::load_config(&config_path)
        .with_context(|| format!("Failed to load configuration from {:?}", config_path))?;

    let de = fin::ui::detect_desktop_environment();
    let (commands, de_columns) = fin::config::get_commands_for_de(&de, &config);
    let commands_clone = commands.clone();
    let config = config.with_columns(de_columns);

    let default_css = PathBuf::from(format!("{}/style.css", fin::config::SYSTEM_CSS_DIR));
    let use_gtk_theme = config.use_gtk_theme;
    let stylesheet_path = fin::css::select_css_path(
        fin::css::css_candidate(
            config.css_path.as_deref(),
            config_path.parent().unwrap_or(Path::new("")),
        ),
        &default_css,
        use_gtk_theme,
    )
    .or_else(|| {
        if use_gtk_theme {
            None
        } else {
            fin::css::load_system_css(&default_css)
        }
    });

    let app = gtk4::Application::builder()
        .application_id("com.fin.launcher")
        .build();

    let config_clone = config.clone();
    app.connect_activate(move |app| {
        if let Err(e) =
            fin::ui::build_ui(app, &config_clone, stylesheet_path.clone(), &commands_clone)
        {
            error!("Error building UI: {:?}", e);
            std::process::exit(1);
        }
    });

    // Run with only argv[0] so the `clap`-consumed `-c`/`--config` flag is not
    // re-parsed (and rejected) by GTK's `GApplication` command-line handling.
    let program = std::env::args().next().unwrap_or_else(|| "fin".to_string());
    app.run_with_args(&[program]);
    Ok(())
}
