use crate::config::{SYSTEM_CONFIG_PATH, SYSTEM_CSS_DIR};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Structure representing the system configuration read from a TOML file.
#[derive(Deserialize)]
struct SystemConfig {
    #[serde(default, alias = "stylesheet")]
    css_path: Option<String>,
}

/// Pure: determine candidate CSS path without checking existence.
#[must_use]
pub fn css_candidate(config_css: Option<&str>, config_dir: &Path) -> Option<PathBuf> {
    config_css.map(|css| {
        let path = PathBuf::from(css);
        if path.is_absolute() {
            path
        } else {
            config_dir.join(css)
        }
    })
}

/// Pure: select between candidate, system, and default paths (no IO beyond existence checks).
#[must_use]
pub fn select_css_path(
    candidate: Option<PathBuf>,
    default_css: &Path,
    use_gtk_theme: bool,
) -> Option<PathBuf> {
    if use_gtk_theme {
        return None;
    }
    candidate
        .filter(|p| p.exists())
        .or_else(|| default_css.exists().then(|| default_css.to_path_buf()))
}

/// Loads the system-configured CSS file, falling back to `default_css` if not found.
pub fn load_system_css(default_css: &Path) -> Option<PathBuf> {
    let system_config_path =
        env::var("FIN_SYSTEM_CONFIG").unwrap_or_else(|_| SYSTEM_CONFIG_PATH.to_string());

    let system_css = (|| {
        let system_config_content = fs::read_to_string(&system_config_path).ok()?;
        let system_config: SystemConfig = toml::from_str(&system_config_content).ok()?;
        let css_str = system_config.css_path?;
        let system_css_path = Path::new(SYSTEM_CSS_DIR).join(css_str);
        if system_css_path.exists() {
            log::info!(
                "Using system-configured CSS at '{}'.",
                system_css_path.display()
            );
            Some(system_css_path)
        } else {
            log::warn!(
                "System-configured CSS '{}' does not exist.",
                system_css_path.display()
            );
            None
        }
    })();

    system_css.or_else(|| {
        if default_css.exists() {
            log::info!(
                "Falling back to default CSS at '{}'.",
                default_css.display()
            );
            Some(default_css.to_path_buf())
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::init_env;
    use std::fs;
    use tempfile::TempDir;

    fn temp() -> TempDir {
        TempDir::new().expect("temp dir")
    }

    #[test]
    fn test_select_css_path_missing_user_css_fallback_to_default() {
        init_env();
        let tmp = temp();
        let config_path = tmp.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config").unwrap();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let default_css = tmp.path().join(format!("default_{}.css", unique));
        fs::write(&default_css, "button { background-color: green; }").unwrap();

        let candidate = css_candidate(Some("nonexistent.css"), config_path.parent().unwrap());
        let result = select_css_path(candidate, &default_css, false);
        assert_eq!(result, Some(default_css.clone()));
    }

    #[test]
    fn test_select_css_path_valid_user_css() {
        init_env();
        let tmp = temp();
        let config_path = tmp.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config").unwrap();
        let user_css_rel = "user_style.css";
        let user_css_path = tmp.path().join(user_css_rel);
        fs::write(&user_css_path, "button { background-color: red; }").unwrap();
        let default_css = tmp.path().join("default.css");
        fs::write(&default_css, "button { background-color: blue; }").unwrap();

        let candidate = css_candidate(Some(user_css_rel), config_path.parent().unwrap());
        let result = select_css_path(candidate, &default_css, false);
        assert_eq!(result, Some(user_css_path.clone()));
    }

    #[test]
    fn test_select_css_path_neither_exist_returns_none() {
        init_env();
        let tmp = temp();
        let config_path = tmp.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config").unwrap();
        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp.path().join("nonexistent_default.css");

        let candidate = css_candidate(user_css.as_deref(), config_path.parent().unwrap());
        let result = select_css_path(candidate, &default_css, false);
        assert_eq!(result, None);
    }

    #[test]
    fn test_select_css_path_use_system_theme_true() {
        init_env();
        let tmp = temp();
        let config_path = tmp.path().join("dummy_config.toml");
        fs::write(&config_path, "dummy config").unwrap();
        let user_css = Some("nonexistent.css".to_string());
        let default_css = tmp.path().join("default.css");
        fs::write(&default_css, "button { background-color: yellow; }").unwrap();

        let candidate = css_candidate(user_css.as_deref(), config_path.parent().unwrap());
        let result = select_css_path(candidate, &default_css, true);
        assert_eq!(result, None);
    }

    #[test]
    fn test_user_config_precedence_and_css_selection() {
        init_env();
        let tmp = temp();
        let tmp_home = tmp.path().join("home");
        fs::create_dir_all(&tmp_home).unwrap();
        env::set_var("HOME", &tmp_home);
        let user_config_dir = tmp_home.join(".config").join("fin");
        fs::create_dir_all(&user_config_dir).unwrap();
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
        fs::write(&user_config_path, user_config_content).unwrap();
        let user_css_path = user_config_dir.join("user_style.css");
        fs::write(&user_css_path, "button { background-color: #ff0000; }").unwrap();

        let determined_path = crate::config::determine_config_path();
        assert_eq!(
            determined_path, user_config_path,
            "The user configuration should take precedence."
        );

        let candidate = css_candidate(Some("user_style.css"), user_config_path.parent().unwrap());
        let css_path = select_css_path(
            candidate,
            &PathBuf::from(format!("{}/style.css", SYSTEM_CSS_DIR)),
            false,
        );
        assert_eq!(
            css_path,
            Some(user_css_path),
            "User-specified stylesheet should be resolved relative to the user config."
        );
    }

    #[test]
    fn test_load_system_css_falls_back_to_default() {
        init_env();
        let tmp = temp();
        // Point the system config at a file whose css path cannot resolve under
        // the hardcoded SYSTEM_CSS_DIR, so load_system_css falls back to default_css.
        let system_config = tmp.path().join("system_config.toml");
        fs::write(&system_config, "stylesheet = \"does_not_exist.css\"\n").unwrap();
        env::set_var("FIN_SYSTEM_CONFIG", &system_config);
        let default_css = tmp.path().join("default.css");
        fs::write(&default_css, "button { background-color: blue; }").unwrap();

        let result = load_system_css(&default_css);
        assert_eq!(result, Some(default_css.clone()));
    }

    #[test]
    fn test_load_system_css_none_when_all_missing() {
        init_env();
        let tmp = temp();
        let system_config = tmp.path().join("system_config.toml");
        fs::write(&system_config, "stylesheet = \"does_not_exist.css\"\n").unwrap();
        env::set_var("FIN_SYSTEM_CONFIG", &system_config);
        let default_css = tmp.path().join("nonexistent_default.css");

        let result = load_system_css(&default_css);
        assert_eq!(result, None);
    }
}
