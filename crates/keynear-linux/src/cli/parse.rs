use std::path::PathBuf;

use keynear_ipc::validate_transfer_id;
use keynear_protocol::{
    default_config_path, sanitize_device_id, validate_display_name, DaemonConfig, KeynearError,
    VisibilityMode,
};

use super::CliAction;

fn validation(field: &'static str, message: String) -> KeynearError {
    KeynearError::Validation { field, message }
}

fn parse_config_path<I>(iter: &mut I) -> Result<PathBuf, KeynearError>
where
    I: Iterator<Item = String>,
{
    let value = iter
        .next()
        .ok_or_else(|| validation("config", "--config requires a file path".to_string()))?;
    if value.trim().is_empty() {
        return Err(validation(
            "config",
            "--config requires a non-empty file path".to_string(),
        ));
    }
    Ok(PathBuf::from(value))
}

fn parse_visibility_arg(raw: &str) -> Result<VisibilityMode, KeynearError> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "hidden" => Ok(VisibilityMode::Hidden),
        "paired" | "paired_only" | "paired-only" => Ok(VisibilityMode::PairedOnly),
        "everyone" | "everyone_nearby" | "everyone-nearby" => Ok(VisibilityMode::EveryoneNearby),
        _ => Err(validation(
            "visibilityMode",
            format!("unknown visibility mode '{raw}' (expected hidden, paired, or everyone)"),
        )),
    }
}

fn ensure_no_prior_action(
    pending: &Option<CliAction>,
    has_config: bool,
) -> Result<(), KeynearError> {
    if pending.is_some() || has_config {
        return Err(validation(
            "args",
            "conflicting CLI commands specified".to_string(),
        ));
    }
    Ok(())
}

fn set_action(
    pending: &mut Option<CliAction>,
    has_config: bool,
    action: CliAction,
) -> Result<(), KeynearError> {
    ensure_no_prior_action(pending, has_config)?;
    *pending = Some(action);
    Ok(())
}

fn parse_rename_device<I>(iter: &mut I) -> Result<CliAction, KeynearError>
where
    I: Iterator<Item = String>,
{
    let raw_device = iter.next().ok_or_else(|| {
        validation(
            "deviceId",
            "--rename-device requires <device_id> <display_name>".to_string(),
        )
    })?;
    let device_id = sanitize_device_id(&raw_device)?;
    let raw_name = iter.next().ok_or_else(|| {
        validation(
            "displayName",
            "--rename-device requires <device_id> <display_name>".to_string(),
        )
    })?;
    let display_name = validate_display_name(&raw_name)?;
    Ok(CliAction::RenameDevice {
        device_id,
        display_name,
    })
}

fn parse_transfer_id_action<I>(iter: &mut I, flag: &str) -> Result<CliAction, KeynearError>
where
    I: Iterator<Item = String>,
{
    let value = iter
        .next()
        .ok_or_else(|| validation("transferId", format!("{flag} requires a transfer_id")))?;
    let clean = validate_transfer_id(&value)?.to_string();
    match flag {
        "--accept" => Ok(CliAction::Accept { transfer_id: clean }),
        "--decline" => Ok(CliAction::Decline { transfer_id: clean }),
        _ => Ok(CliAction::Cancel { transfer_id: clean }),
    }
}

fn parse_send<I>(iter: &mut I) -> Result<CliAction, KeynearError>
where
    I: Iterator<Item = String>,
{
    let raw_device = iter.next().ok_or_else(|| {
        validation(
            "deviceId",
            "--send requires <device_id> <path>...".to_string(),
        )
    })?;
    let device_id = sanitize_device_id(&raw_device)?;
    let mut paths = Vec::new();
    let mut literal_paths = false;
    for operand in iter.by_ref() {
        if !literal_paths && operand == "--" {
            literal_paths = true;
            continue;
        }
        if !literal_paths && operand.starts_with('-') {
            return Err(validation(
                "args",
                format!(
                    "unexpected flag '{operand}' after --send (use '--' before paths starting with '-')"
                ),
            ));
        }
        if operand.trim().is_empty() {
            return Err(validation(
                "paths",
                "--send paths must not be empty".to_string(),
            ));
        }
        paths.push(PathBuf::from(operand));
    }
    if paths.is_empty() {
        return Err(validation(
            "paths",
            "--send requires at least one path".to_string(),
        ));
    }
    Ok(CliAction::Send { device_id, paths })
}

fn parse_config_arg<I>(iter: &mut I, pending: &Option<CliAction>) -> Result<PathBuf, KeynearError>
where
    I: Iterator<Item = String>,
{
    if pending.is_some() {
        return Err(validation(
            "args",
            "--config cannot be combined with transfer commands".to_string(),
        ));
    }
    parse_config_path(iter)
}

fn parse_visibility_action<I>(
    iter: &mut I,
    pending: &Option<CliAction>,
    has_config: bool,
) -> Result<CliAction, KeynearError>
where
    I: Iterator<Item = String>,
{
    ensure_no_prior_action(pending, has_config)?;
    let value = iter.next().ok_or_else(|| {
        validation(
            "visibilityMode",
            "--set-visibility requires hidden, paired, or everyone".to_string(),
        )
    })?;
    let mode = parse_visibility_arg(&value)?;
    Ok(CliAction::SetVisibility { mode })
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    let mut config_path: Option<PathBuf> = None;
    let mut pending_action: Option<CliAction> = None;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--version" | "-V" => return Ok(CliAction::Version),
            "--help" | "-h" => return Ok(CliAction::Help),
            "--config" => {
                config_path = Some(parse_config_arg(&mut iter, &pending_action)?);
            }
            "--transfers" => set_action(
                &mut pending_action,
                config_path.is_some(),
                CliAction::Transfers,
            )?,
            "--status" => set_action(
                &mut pending_action,
                config_path.is_some(),
                CliAction::Status,
            )?,
            "--devices" => set_action(
                &mut pending_action,
                config_path.is_some(),
                CliAction::Devices,
            )?,
            "--set-visibility" => {
                pending_action = Some(parse_visibility_action(
                    &mut iter,
                    &pending_action,
                    config_path.is_some(),
                )?);
            }
            "--rename-device" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                pending_action = Some(parse_rename_device(&mut iter)?);
            }
            flag @ ("--accept" | "--decline" | "--cancel") => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                pending_action = Some(parse_transfer_id_action(&mut iter, flag)?);
            }
            "--send" => {
                ensure_no_prior_action(&pending_action, config_path.is_some())?;
                pending_action = Some(parse_send(&mut iter)?);
            }
            other => {
                return Err(validation("args", format!("unknown argument '{other}'")));
            }
        }
    }

    Ok(pending_action.unwrap_or(CliAction::Run { config_path }))
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
