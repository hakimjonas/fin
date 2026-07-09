use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Theme error: {0}")]
    Theme(#[from] ThemeError),

    #[error("CSS error: {0}")]
    Css(String),

    #[error("UI build error: {0}")]
    Ui(String),

    #[error("Command execution failed: {0}")]
    Command(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    Read(#[from] std::io::Error),

    #[error("TOML deserialization error: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("Invalid configuration: {0}")]
    Validation(String),
}

#[derive(Error, Debug)]
pub enum ThemeError {
    #[error("Failed to read theme file: {0}")]
    Read(#[from] std::io::Error),

    #[error("Theme TOML deserialization error: {0}")]
    Parse(#[from] toml::de::Error),
}
