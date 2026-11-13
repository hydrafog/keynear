mod ble;
mod service;

use std::path::PathBuf;
use std::process;

use keynear_core::{default_config_path, DaemonConfig, KeynearError};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliAction {
    Version,
    Help,
    Run { config_path: Option<PathBuf> },
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    let mut config_path: Option<PathBuf> = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--version" | "-V" => return Ok(CliAction::Version),
            "--help" | "-h" => return Ok(CliAction::Help),
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

    Ok(CliAction::Run { config_path })
}

pub fn load_daemon_config(explicit_path: Option<&PathBuf>) -> Result<DaemonConfig, KeynearError> {
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

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let action = match parse_cli_args(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(2);
        }
    };

    match action {
        CliAction::Version => {
            println!("keynear-linux {}", env!("CARGO_PKG_VERSION"));
        }
        CliAction::Help => {
            println!(
                "Usage: keynear-linux [--config <path>] [--version] [--help]\n\nOptions:\n  --config <path>  Path to TOML daemon configuration\n  --version, -V    Print version information\n  --help, -h       Print this help message"
            );
        }
        CliAction::Run { config_path } => {
            let config = match load_daemon_config(config_path.as_ref()) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
            };
            if let Err(err) = service::DaemonRuntime::run_until_shutdown(&config).await {
                eprintln!("error: {err}");
                process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn cli_version_and_help_parsed() {
        assert_eq!(parse_cli_args(["--version"]).unwrap(), CliAction::Version);
        assert_eq!(parse_cli_args(["-h"]).unwrap(), CliAction::Help);
    }

    #[test]
    fn cli_config_path_parsed() {
        assert_eq!(
            parse_cli_args(["--config", "/tmp/keynear.toml"]).unwrap(),
            CliAction::Run {
                config_path: Some(PathBuf::from("/tmp/keynear.toml"))
            }
        );
    }

    #[test]
    fn cli_missing_config_value_fails() {
        let err = parse_cli_args(["--config"]).expect_err("missing --config operand must fail");
        assert!(matches!(
            err,
            KeynearError::Validation {
                field: "config",
                ..
            }
        ));
    }

    #[test]
    fn cli_unknown_flag_fails() {
        let err = parse_cli_args(["--bogus"]).expect_err("unknown flag must fail");
        assert!(matches!(
            err,
            KeynearError::Validation { field: "args", .. }
        ));
    }

    #[test]
    fn explicit_missing_config_file_rejected() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let missing = std::env::temp_dir().join(format!("keynear-linux-missing-{nanos}.toml"));
        let err =
            load_daemon_config(Some(&missing)).expect_err("missing explicit config must fail");
        assert!(matches!(err, KeynearError::Config(_)));
    }
}
