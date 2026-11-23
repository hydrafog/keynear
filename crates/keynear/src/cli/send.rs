use std::path::PathBuf;

use keynear_protocol::{sanitize_device_id, KeynearError};

use super::CliAction;

fn validation(field: &'static str, message: String) -> KeynearError {
    KeynearError::Validation { field, message }
}

fn parse_device_value<I>(iter: &mut I) -> Result<String, KeynearError>
where
    I: Iterator<Item = String>,
{
    let raw = iter
        .next()
        .ok_or_else(|| validation("deviceId", "--device requires a device_id".to_string()))?;
    sanitize_device_id(&raw)
}

fn push_checked_path(
    paths: &mut Vec<PathBuf>,
    raw: &str,
    context: &str,
) -> Result<(), KeynearError> {
    if raw.trim().is_empty() {
        return Err(validation(
            "paths",
            format!("{context} paths must not be empty"),
        ));
    }
    paths.push(PathBuf::from(raw));
    Ok(())
}

pub(super) fn parse_legacy_send<I>(iter: &mut I) -> Result<CliAction, KeynearError>
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
        push_checked_path(&mut paths, &operand, "--send")?;
    }
    if paths.is_empty() {
        return Err(validation(
            "paths",
            "--send requires at least one path".to_string(),
        ));
    }
    Ok(CliAction::Send {
        device_id: Some(device_id),
        paths,
    })
}

pub(super) fn parse_send_picker<I>(iter: &mut I) -> Result<CliAction, KeynearError>
where
    I: Iterator<Item = String>,
{
    let mut device_id = None;
    let mut directory = false;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => return Ok(CliAction::Help),
            "--folder" | "--directory" => directory = true,
            "--device" => {
                if device_id.is_some() {
                    return Err(validation(
                        "deviceId",
                        "duplicate --device flag".to_string(),
                    ));
                }
                device_id = Some(parse_device_value(iter)?);
            }
            other => {
                return Err(validation(
                    "args",
                    format!("unexpected argument '{other}' for --send-picker"),
                ));
            }
        }
    }
    Ok(CliAction::SendPicker {
        device_id,
        directory,
    })
}

#[derive(Default)]
struct SendArgsState {
    device_id: Option<String>,
    picker: bool,
    directory: bool,
    paths: Vec<PathBuf>,
    literal_paths: bool,
}

fn step_send_token<I>(
    arg: String,
    iter: &mut I,
    state: &mut SendArgsState,
    allow_positional_device: bool,
) -> Result<Option<CliAction>, KeynearError>
where
    I: Iterator<Item = String>,
{
    if state.literal_paths {
        push_checked_path(&mut state.paths, &arg, "send")?;
        return Ok(None);
    }
    match arg.as_str() {
        "--help" | "-h" => return Ok(Some(CliAction::Help)),
        "--" => state.literal_paths = true,
        "--picker" if allow_positional_device => state.picker = true,
        "--folder" | "--directory" if allow_positional_device => state.directory = true,
        "--device" => {
            if state.device_id.is_some() {
                return Err(validation(
                    "deviceId",
                    "duplicate --device flag".to_string(),
                ));
            }
            state.device_id = Some(parse_device_value(iter)?);
        }
        other if other.starts_with('-') => {
            return Err(validation(
                "args",
                format!(
                    "unexpected flag '{other}' after send (use '--' before paths starting with '-')"
                ),
            ));
        }
        positional => {
            if allow_positional_device && state.device_id.is_none() && state.paths.is_empty() {
                state.device_id = Some(sanitize_device_id(positional)?);
            } else {
                push_checked_path(&mut state.paths, positional, "send")?;
            }
        }
    }
    Ok(None)
}

fn finalize_send_state(state: SendArgsState, flag_name: &str) -> Result<CliAction, KeynearError> {
    if state.picker {
        if state.literal_paths || !state.paths.is_empty() {
            return Err(validation(
                "args",
                "--picker cannot be combined with file paths".to_string(),
            ));
        }
        return Ok(CliAction::SendPicker {
            device_id: state.device_id,
            directory: state.directory,
        });
    }
    if state.directory {
        return Err(validation("args", "--folder requires --picker".to_string()));
    }
    if state.paths.is_empty() {
        return Err(validation(
            "paths",
            format!("{flag_name} requires at least one file or directory path"),
        ));
    }
    Ok(CliAction::Send {
        device_id: state.device_id,
        paths: state.paths,
    })
}

pub(super) fn parse_legacy_share<I>(iter: &mut I) -> Result<CliAction, KeynearError>
where
    I: Iterator<Item = String>,
{
    let mut state = SendArgsState::default();
    while let Some(arg) = iter.next() {
        if let Some(early) = step_send_token(arg, iter, &mut state, false)? {
            return Ok(early);
        }
    }
    finalize_send_state(state, "--share")
}

pub(super) fn parse_legacy_send_flag(
    arg: &str,
    iter: &mut impl Iterator<Item = String>,
) -> Result<Option<CliAction>, KeynearError> {
    match arg {
        "--send" => parse_legacy_send(iter).map(Some),
        "--send-picker" => parse_send_picker(iter).map(Some),
        "--share" => parse_legacy_share(iter).map(Some),
        _ => Ok(None),
    }
}

pub(super) fn parse_send_subcommand<I>(iter: &mut I) -> Result<CliAction, KeynearError>
where
    I: Iterator<Item = String>,
{
    let mut state = SendArgsState::default();
    while let Some(arg) = iter.next() {
        if let Some(early) = step_send_token(arg, iter, &mut state, true)? {
            return Ok(early);
        }
    }
    finalize_send_state(state, "send")
}

#[cfg(test)]
mod tests {
    use crate::cli::{parse_cli_args, CliAction};
    use keynear_protocol::KeynearError;

    #[test]
    fn cli_send_folder_picker_flags_parsed() {
        assert_eq!(
            parse_cli_args(["send", "--picker", "--folder", "--device", "pixel-9"]).unwrap(),
            CliAction::SendPicker {
                device_id: Some("pixel-9".into()),
                directory: true,
            }
        );
        assert_eq!(
            parse_cli_args(["--send-picker", "--directory", "--device", "pixel-9"]).unwrap(),
            CliAction::SendPicker {
                device_id: Some("pixel-9".into()),
                directory: true,
            }
        );
        assert!(matches!(
            parse_cli_args(["send", "--folder", "--", "/tmp/a"]),
            Err(KeynearError::Validation { field: "args", .. })
        ));
    }
}
