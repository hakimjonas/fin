use crate::config::Config;
use crate::error::ThemeError;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

pub const SYSTEM_THEME_DIR: &str = "/usr/share/fin/themes";

/// A type‑safe, immutable representation of the theme colors.
/// The TOML file is expected to use hyphenated keys.
#[derive(Debug, Deserialize)]
pub struct ThemeColors {
    #[serde(rename = "palette0")]
    pub palette0: String, // Base
    #[serde(rename = "palette1")]
    pub palette1: String, // Surface
    #[serde(rename = "palette2")]
    pub palette2: String, // Overlay
    #[serde(rename = "palette3")]
    pub palette3: String, // Muted
    #[serde(rename = "palette4")]
    pub palette4: String, // Subtle
    #[serde(rename = "palette5")]
    pub palette5: String, // Text
    #[serde(rename = "palette6")]
    pub palette6: String, // Love
    #[serde(rename = "palette7")]
    pub palette7: String, // Gold
    #[serde(rename = "palette8")]
    pub palette8: String, // Rose
    #[serde(rename = "palette9")]
    pub palette9: String, // Pine
    #[serde(rename = "palette10")]
    pub palette10: String, // Foam
    #[serde(rename = "palette11")]
    pub palette11: String, // Iris
    #[serde(rename = "palette12")]
    pub palette12: String, // Highlight Low
    #[serde(rename = "palette13")]
    pub palette13: String, // Highlight Med
    #[serde(rename = "palette14")]
    pub palette14: String, // Highlight High
    #[serde(rename = "palette15")]
    pub palette15: String, // Highlight Text

    #[serde(rename = "background")]
    pub background: String, // Window background
    #[serde(rename = "foreground")]
    pub foreground: String, // Window foreground

    #[serde(rename = "button-focus-text")]
    pub button_focus_text: String, // Button focus text color
    #[serde(rename = "button-focus-background")]
    pub button_focus_background: String, // Button focus background
    #[serde(rename = "button-hover-background")]
    pub button_hover_background: String, // Button hover background
    #[serde(rename = "button-hover-text")]
    pub button_hover_text: String, // Button hover text
    #[serde(rename = "button-normal-background")]
    pub button_normal_background: String, // Normal button background
}

/// Pure: parse a TOML string into `ThemeColors`.
pub fn parse_theme(content: &str) -> Result<ThemeColors, ThemeError> {
    Ok(toml::from_str(content)?)
}

/// IO: read a file and delegate to `parse_theme`.
pub fn load_theme_colors(path: &Path) -> Result<ThemeColors, ThemeError> {
    let content = fs::read_to_string(path)?;
    parse_theme(&content)
}

/// Generates a CSS string that maps the theme colors to CSS custom properties.
#[must_use]
pub fn generate_theme_css(theme: &ThemeColors) -> String {
    format!(
        ":root {{
  --palette0: {};
  --palette1: {};
  --palette2: {};
  --palette3: {};
  --palette4: {};
  --palette5: {};
  --palette6: {};
  --palette7: {};
  --palette8: {};
  --palette9: {};
  --palette10: {};
  --palette11: {};
  --palette12: {};
  --palette13: {};
  --palette14: {};
  --palette15: {};

  --background: {};
  --foreground: {};

  --button-focus-text: {};
  --button-focus-background: {};
  --button-hover-background: {};
  --button-hover-text: {};
  --button-normal-background: {};
}}
button {{
  background: var(--button-normal-background);
  color: var(--button-focus-text);
}}
",
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
        theme.background,
        theme.foreground,
        theme.button_focus_text,
        theme.button_focus_background,
        theme.button_hover_background,
        theme.button_hover_text,
        theme.button_normal_background,
    )
}

/// Pure: resolve the candidate theme file path (user dir takes precedence over system).
#[must_use]
fn resolve_theme_path(theme_name: &str) -> Option<PathBuf> {
    let user_path = dirs::config_dir()
        .map(|p| {
            p.join("fin")
                .join("themes")
                .join(format!("{}.toml", theme_name))
        })
        .filter(|p| p.exists());
    let system_path = PathBuf::from(SYSTEM_THEME_DIR).join(format!("{}.toml", theme_name));
    user_path.or_else(|| {
        if system_path.exists() {
            Some(system_path)
        } else {
            None
        }
    })
}

/// Thin IO wrapper: load the theme for the config and generate its CSS string.
/// On any failure, returns an empty string (logging happens at the boundary).
#[must_use]
pub fn get_theme_css(config: &Config) -> String {
    let theme_name = config.theme.as_deref().unwrap_or("default");
    resolve_theme_path(theme_name)
        .and_then(|p| load_theme_colors(&p).ok())
        .map(|theme| generate_theme_css(&theme))
        .unwrap_or_else(|| {
            log::error!("Failed to load theme '{}'", theme_name);
            String::new()
        })
}
