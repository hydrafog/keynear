use std::path::PathBuf;

use keynear_ipc::validate_transfer_id;
use keynear_protocol::{sanitize_device_id, validate_display_name, KeynearError};

use super::send::{parse_legacy_send_flag, parse_send_subcommand};
use super::{parse_config_path, parse_visibility_arg, CliAction};

fn validation(field: &'static str, message: String) -> KeynearError {
    KeynearError::Validation { field, message }
}

fn next_arg_or_help(
    iter: &mut impl Iterator<Item = String>,
    field: &'static str,
    msg: &str,
) -> Result<Option<String>, KeynearError> {
    let value = iter
        .next()
        .ok_or_else(|| validation(field, msg.to_string()))?;
    if matches!(value.as_str(), "--help" | "-h") {
        return Ok(None);
    }
    Ok(Some(value))
}

fn expect_no_more_args(
    iter: &mut impl Iterator<Item = String>,
    action: CliAction,
) -> Result<CliAction, KeynearError> {
    let Some(extra) = iter.next() else {
        return Ok(action);
    };
    if matches!(extra.as_str(), "--help" | "-h") {
        return Ok(CliAction::Help);
    }
    Err(validation("args", format!("unexpected argument '{extra}'")))
}

fn parse_rename_args(
    iter: &mut impl Iterator<Item = String>,
    cmd_name: &str,
) -> Result<CliAction, KeynearError> {
    let msg = format!("{cmd_name} requires <device_id> <display_name>");
    let Some(raw_device) = next_arg_or_help(iter, "deviceId", &msg)? else {
        return Ok(CliAction::Help);
    };
    let device_id = sanitize_device_id(&raw_device)?;
    let raw_name = iter.next().ok_or_else(|| validation("displayName", msg))?;
    let display_name = validate_display_name(&raw_name)?;
    expect_no_more_args(
        iter,
        CliAction::RenameDevice {
            device_id,
            display_name,
        },
    )
}

fn parse_transfer_id_action(
    iter: &mut impl Iterator<Item = String>,
    cmd_name: &str,
    kind: &str,
) -> Result<CliAction, KeynearError> {
    let msg = format!("{cmd_name} requires a transfer_id");
    let Some(value) = next_arg_or_help(iter, "transferId", &msg)? else {
        return Ok(CliAction::Help);
    };
    let transfer_id = validate_transfer_id(&value)?.to_string();
    let action = match kind {
        "accept" => CliAction::Accept { transfer_id },
        "decline" => CliAction::Decline { transfer_id },
        _ => CliAction::Cancel { transfer_id },
    };
    expect_no_more_args(iter, action)
}

fn parse_visibility_value(
    iter: &mut impl Iterator<Item = String>,
    cmd_name: &str,
) -> Result<CliAction, KeynearError> {
    let msg = format!("{cmd_name} requires hidden, paired, or everyone");
    let Some(value) = next_arg_or_help(iter, "visibilityMode", &msg)? else {
        return Ok(CliAction::Help);
    };
    let mode = parse_visibility_arg(&value)?;
    expect_no_more_args(iter, CliAction::SetVisibility { mode })
}

fn assign_config(
    slot: &mut Option<PathBuf>,
    iter: &mut impl Iterator<Item = String>,
) -> Result<(), KeynearError> {
    if slot.is_some() {
        return Err(validation("config", "duplicate --config flag".into()));
    }
    *slot = Some(parse_config_path(iter)?);
    Ok(())
}

fn parse_daemon_subcommand(
    iter: &mut impl Iterator<Item = String>,
) -> Result<CliAction, KeynearError> {
    let mut config_path = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(CliAction::Help),
            "--config" => assign_config(&mut config_path, iter)?,
            other => return Err(validation("args", format!("unexpected argument '{other}'"))),
        }
    }
    Ok(CliAction::Run { config_path })
}

fn parse_settings_subcommand(
    iter: &mut impl Iterator<Item = String>,
) -> Result<CliAction, KeynearError> {
    let mut config_path = None;
    let mut validate_only: Option<bool> = None;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(CliAction::Help),
            "show" if validate_only.is_none() => validate_only = Some(false),
            "validate" if validate_only.is_none() => validate_only = Some(true),
            "show" | "validate" => {
                return Err(validation("args", "conflicting settings mode".into()));
            }
            "--config" => assign_config(&mut config_path, iter)?,
            other => return Err(validation("args", format!("unexpected argument '{other}'"))),
        }
    }
    Ok(CliAction::Settings {
        config_path,
        validate_only: validate_only.unwrap_or(false),
    })
}

fn parse_subcommand(
    cmd: &str,
    iter: &mut impl Iterator<Item = String>,
) -> Result<CliAction, KeynearError> {
    match cmd {
        "daemon" => parse_daemon_subcommand(iter),
        "settings" => parse_settings_subcommand(iter),
        "status" => expect_no_more_args(iter, CliAction::Status),
        "devices" => expect_no_more_args(iter, CliAction::Devices),
        "transfers" => expect_no_more_args(iter, CliAction::Transfers),
        "send" => parse_send_subcommand(iter),
        "accept" => parse_transfer_id_action(iter, "accept", "accept"),
        "decline" => parse_transfer_id_action(iter, "decline", "decline"),
        "cancel" => parse_transfer_id_action(iter, "cancel", "cancel"),
        "visibility" => parse_visibility_value(iter, "visibility"),
        "rename" => parse_rename_args(iter, "rename"),
        "version" => expect_no_more_args(iter, CliAction::Version),
        "help" => Ok(CliAction::Help),
        other => Err(validation("args", format!("unknown command '{other}'"))),
    }
}

#[derive(Default)]
struct LegacyArgsState {
    config_path: Option<PathBuf>,
    validate_only: bool,
    pending_action: Option<CliAction>,
}

fn conflict_err() -> KeynearError {
    validation("args", "conflicting CLI commands specified".into())
}

fn check_no_legacy_conflict(state: &LegacyArgsState) -> Result<(), KeynearError> {
    if state.pending_action.is_some() || state.config_path.is_some() || state.validate_only {
        return Err(conflict_err());
    }
    Ok(())
}

fn apply_legacy_mode_flag(
    arg: &str,
    iter: &mut impl Iterator<Item = String>,
    state: &mut LegacyArgsState,
) -> Result<bool, KeynearError> {
    match arg {
        "--config" if state.pending_action.is_none() => {
            state.config_path = Some(parse_config_path(iter)?);
            Ok(true)
        }
        "--validate" if state.pending_action.is_none() => {
            state.validate_only = true;
            Ok(true)
        }
        "--config" | "--validate" => Err(conflict_err()),
        _ => Ok(false),
    }
}

fn parse_legacy_action_flag(
    arg: &str,
    iter: &mut impl Iterator<Item = String>,
) -> Result<Option<CliAction>, KeynearError> {
    let action = match arg {
        "--transfers" => CliAction::Transfers,
        "--status" => CliAction::Status,
        "--devices" => CliAction::Devices,
        "--set-visibility" => parse_visibility_value(iter, "--set-visibility")?,
        "--rename-device" => parse_rename_args(iter, "--rename-device")?,
        "--accept" | "--decline" | "--cancel" => {
            parse_transfer_id_action(iter, arg, arg.trim_start_matches('-'))?
        }
        _ => return Ok(None),
    };
    Ok(Some(action))
}

fn step_legacy_flag(
    arg: &str,
    iter: &mut impl Iterator<Item = String>,
    state: &mut LegacyArgsState,
) -> Result<Option<CliAction>, KeynearError> {
    if matches!(arg, "--version" | "-V") {
        return Ok(Some(CliAction::Version));
    }
    if matches!(arg, "--help" | "-h") {
        return Ok(Some(CliAction::Help));
    }
    if apply_legacy_mode_flag(arg, iter, state)? {
        return Ok(None);
    }
    if let Some(action) = parse_legacy_action_flag(arg, iter)? {
        check_no_legacy_conflict(state)?;
        state.pending_action = Some(action);
        return Ok(None);
    }
    if let Some(send_action) = parse_legacy_send_flag(arg, iter)? {
        check_no_legacy_conflict(state)?;
        return Ok(Some(send_action));
    }
    Err(validation("args", format!("unknown argument '{arg}'")))
}

fn parse_legacy_flag_args(
    first: String,
    rest: impl Iterator<Item = String>,
) -> Result<CliAction, KeynearError> {
    let mut state = LegacyArgsState::default();
    let mut iter = std::iter::once(first).chain(rest);
    while let Some(arg) = iter.next() {
        if let Some(early) = step_legacy_flag(&arg, &mut iter, &mut state)? {
            return Ok(early);
        }
    }
    if state.validate_only {
        return Ok(CliAction::Settings {
            config_path: state.config_path,
            validate_only: true,
        });
    }
    Ok(state.pending_action.unwrap_or(CliAction::Run {
        config_path: state.config_path,
    }))
}

pub fn parse_cli_args<I, S>(args: I) -> Result<CliAction, KeynearError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut iter = args.into_iter().map(Into::into);
    let Some(first) = iter.next() else {
        return Ok(CliAction::Run { config_path: None });
    };
    if first.starts_with('-') {
        parse_legacy_flag_args(first, iter)
    } else {
        parse_subcommand(&first, &mut iter)
    }
}
