mod app;

use std::path::PathBuf;
use std::process;

use keynear_core::{default_config_path, DaemonConfig, KeynearError};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsCliAction {
    Version,
    Help,
    Validate { config_path: Option<PathBuf> },
    Run { config_path: Option<PathBuf> },
}

pub fn parse_settings_args<I, S>(args: I) -> Result<SettingsCliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    let mut config_path: Option<PathBuf> = None;
    let mut validate_only = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--version" | "-V" => return Ok(SettingsCliAction::Version),
            "--help" | "-h" => return Ok(SettingsCliAction::Help),
            "--validate" => validate_only = true,
            "--config" => {
                let value = iter.next().ok_or_else(|| KeynearError::Validation {
                    field: "config",
                    message: "--config requires a file path".to_string(),
                })?;
                if value.trim().is_empty() {
                    return Err(KeynearError::Validation {
                        field: "config",
                        message: "--config requires a non-empty file path".to_string(),
                    });
                }
                config_path = Some(PathBuf::from(value));
            }
            other => {
                return Err(KeynearError::Validation {
                    field: "args",
                    message: format!("unknown argument '{other}'"),
                });
            }
        }
    }

    if validate_only {
        Ok(SettingsCliAction::Validate { config_path })
    } else {
        Ok(SettingsCliAction::Run { config_path })
    }
}

pub fn load_settings_config(explicit_path: Option<&PathBuf>) -> Result<DaemonConfig, KeynearError> {
    match explicit_path {
        Some(path) => {
            if !path.exists() {
                return Err(KeynearError::Config(format!(
                    "configuration file not found: {}",
                    path.display()
                )));
            }
            DaemonConfig::load_or_default(path)
        }
        None => DaemonConfig::load_or_default(&default_config_path()),
    }
}

pub fn validate_css_tokens() -> Result<(), KeynearError> {
    let required = [
        "#F0F4FA",
        "#0B1016",
        "#004CED",
        "#0172F9",
        "Plus Jakarta Sans",
        "min-height: 44px",
        ".keynear-window",
        ".keynear-card",
        ".keynear-error",
    ];
    for needle in required {
        if !app::STYLE_CSS.contains(needle) {
            return Err(KeynearError::Validation {
                field: "style_css",
                message: format!("missing required design token or selector '{needle}'"),
            });
        }
    }
    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let action = match parse_settings_args(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(2);
        }
    };

    match action {
        SettingsCliAction::Version => {
            println!("keynear-settings {}", env!("CARGO_PKG_VERSION"));
        }
        SettingsCliAction::Help => {
            println!("Usage: keynear-settings [--config <path>] [--validate] [--version] [--help]");
        }
        SettingsCliAction::Validate { config_path } => {
            if let Err(err) = validate_css_tokens() {
                eprintln!("error: {err}");
                process::exit(1);
            }
            let config = match load_settings_config(config_path.as_ref()) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
            };
            info!(
                event = "settings_validated",
                visibility = ?config.visibility_mode,
                incoming_dir = %config.incoming_directory
            );
        }
        SettingsCliAction::Run { config_path } => {
            if let Err(err) = validate_css_tokens() {
                eprintln!("error: {err}");
                process::exit(1);
            }
            let (config, error_message) = match load_settings_config(config_path.as_ref()) {
                Ok(config) => (config, None),
                Err(err) => (DaemonConfig::default(), Some(err.to_string())),
            };
            info!(
                event = "settings_launching",
                visibility = ?config.visibility_mode,
                has_error = error_message.is_some()
            );
            app::run_gui(app::SettingsInit {
                config,
                error_message,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_validate_and_config() {
        assert_eq!(
            parse_settings_args(["--validate", "--config", "/tmp/a.toml"]).unwrap(),
            SettingsCliAction::Validate {
                config_path: Some(PathBuf::from("/tmp/a.toml"))
            }
        );
        assert_eq!(
            parse_settings_args(["--version"]).unwrap(),
            SettingsCliAction::Version
        );
    }

    #[test]
    fn cli_rejects_missing_config_value() {
        let err =
            parse_settings_args(["--config"]).expect_err("missing --config operand must fail");
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "config",
                ..
            }
        ));
    }

    #[test]
    fn css_tokens_match_design_tokens_json() {
        validate_css_tokens().expect("css tokens valid");
        let raw_json = include_str!("../../../design/tokens.json");
        let parsed: serde_json::Value =
            serde_json::from_str(raw_json).expect("parse design/tokens.json");
        let light_canvas = parsed["light"]["canvas"].as_str().unwrap();
        let dark_canvas = parsed["dark"]["canvas"].as_str().unwrap();
        let light_accent = parsed["light"]["accent"].as_str().unwrap();
        let focus_ring = parsed["light"]["focusRing"].as_str().unwrap();

        assert!(app::STYLE_CSS.contains(light_canvas));
        assert!(app::STYLE_CSS.contains(dark_canvas));
        assert!(app::STYLE_CSS.contains(light_accent));
        assert!(app::STYLE_CSS.contains(focus_ring));
    }

    #[test]
    fn headless_validate_mode_loads_default_config_and_css() {
        validate_css_tokens().expect("css tokens valid");
        let config = DaemonConfig::default();
        assert!(config.validate().is_ok());
    }
}
