use std::path::PathBuf;
use std::process;

use keynear_ipc::{daemon_socket_path, send_request, IpcRequest, IpcResponse};
use keynear_protocol::{default_config_path, DaemonConfig};
use keynear_settings::{
    app, load_device_store_snapshot, load_settings_config, parse_settings_args,
    validate_css_tokens, SettingsCliAction,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

fn launch_settings_gui(
    config_path: Option<PathBuf>,
    send_picker_active: bool,
    selected_send_device_id: Option<String>,
    send_paths: Vec<PathBuf>,
) {
    if let Err(err) = validate_css_tokens() {
        eprintln!("error: {err}");
        process::exit(1);
    }
    let resolved_config_path = config_path.clone().unwrap_or_else(default_config_path);
    let (mut config, error_message) = match load_settings_config(config_path.as_ref()) {
        Ok(config) => (config, None),
        Err(err) => (DaemonConfig::default(), Some(err.to_string())),
    };
    let (device_store, mut paired_devices, warning_message) = load_device_store_snapshot();
    let mut everyone_remaining_secs = None;
    let mut transfer_error = None;

    match send_request(&IpcRequest::Status) {
        Ok(IpcResponse::Status(status)) => {
            config.visibility_mode = status.visibility_mode;
            config.auto_accept_from_paired = status.auto_accept_from_paired;
            config.incoming_directory = status.incoming_directory;
            everyone_remaining_secs = status.everyone_remaining_secs;
        }
        Ok(_) => {}
        Err(err) if send_picker_active => {
            transfer_error = Some(format!(
                "Keynear daemon unavailable ({err}); start keynear-linux and retry."
            ));
        }
        Err(_) => {}
    }

    if let Ok(IpcResponse::Config(daemon_cfg)) = send_request(&IpcRequest::GetConfig) {
        config = daemon_cfg;
    }
    if let Ok(IpcResponse::Devices(devices)) = send_request(&IpcRequest::ListDevices) {
        paired_devices = devices;
    }
    let initial_transfers = match send_request(&IpcRequest::ListTransfers) {
        Ok(IpcResponse::Transfers(transfers)) => transfers,
        _ => Vec::new(),
    };

    info!(
        event = "settings_launching",
        visibility = ?config.visibility_mode,
        paired_count = paired_devices.len(),
        send_picker_active,
        staged_send_count = send_paths.len(),
        has_error = error_message.is_some()
    );

    app::run_gui(app::SettingsInit {
        config,
        config_path: Some(resolved_config_path),
        everyone_remaining_secs,
        error_message,
        warning_message,
        transfer_error,
        paired_devices,
        nearby_peers: Vec::new(),
        transfers: initial_transfers,
        send_paths,
        selected_send_device_id,
        send_picker_active,
        device_store,
        ipc_socket_path: Some(daemon_socket_path()),
    });
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
            println!(
                "Usage: keynear-settings [--config <path>] [--validate] [--version] [--help]\n       keynear-settings --send-picker [--device <device_id>] [--config <path>]\n       keynear-settings --share [--device <device_id>] -- <path>..."
            );
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
            launch_settings_gui(config_path, false, None, Vec::new());
        }
        SettingsCliAction::SendPicker {
            config_path,
            device_id,
        } => {
            launch_settings_gui(config_path, true, device_id, Vec::new());
        }
        SettingsCliAction::Share {
            config_path,
            device_id,
            paths,
        } => {
            launch_settings_gui(config_path, true, device_id, paths);
        }
    }
}
