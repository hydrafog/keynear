#![deny(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments
)]

// TODO: drive VegaAudioController from the peer audio-control stream once its QUIC demux lands.
mod audio;
mod ble;
mod call;
mod cli;
mod clipboard;
mod fallback;
mod fuse;
mod ipc;
mod keyring_oo7;
mod media;
mod notifications;
mod portal;
mod service;
mod tray;
mod vega;

use std::path::PathBuf;
use std::process;

use keynear_ipc::{daemon_socket_path, IpcRequest};
use keynear_protocol::KeynearError;
use tracing_subscriber::EnvFilter;

use crate::cli::{
    cli_action_to_ipc_request, load_daemon_config, parse_cli_args, run_ipc_command,
    run_settings_command, CliAction, CLI_USAGE_TEXT,
};

fn env_filter() -> EnvFilter {
    if let Ok(value) = std::env::var("KEYNEAR_LOG") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            if let Ok(filter) = EnvFilter::try_new(trimmed) {
                return filter;
            }
        }
    }
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_env_filter(env_filter()).init();

    let action = match parse_cli_args(std::env::args().skip(1)) {
        Ok(action) => action,
        Err(err) => {
            eprintln!("error: {err}");
            process::exit(2);
        }
    };

    if let Some(request) = cli_action_to_ipc_request(&action) {
        handle_ipc_request(request).await;
        return;
    }

    dispatch_action(action).await;
}

fn exit_with_error(err: impl std::fmt::Display) -> ! {
    eprintln!("error: {err}");
    process::exit(1);
}

async fn run_blocking_ipc<F, T>(task: F) -> T
where
    F: FnOnce() -> Result<T, KeynearError> + Send + 'static,
    T: Send + 'static,
{
    match tokio::task::spawn_blocking(task).await {
        Ok(Ok(value)) => value,
        Ok(Err(err)) => exit_with_error(err),
        Err(err) => exit_with_error(format!("command task failed: {err}")),
    }
}

async fn handle_ipc_request(request: IpcRequest) {
    let sock = daemon_socket_path();
    let output = run_blocking_ipc(move || run_ipc_command(&sock, &request)).await;
    println!("{output}");
}

async fn run_daemon(config_path: Option<PathBuf>) {
    let config = match load_daemon_config(config_path.as_ref()) {
        Ok(config) => config,
        Err(err) => exit_with_error(err),
    };
    if let Err(err) =
        service::DaemonRuntime::run_until_shutdown_with_config_path(&config, config_path).await
    {
        exit_with_error(err);
    }
}

async fn run_send_picker(device_id: Option<String>) {
    let sock = daemon_socket_path();
    let target = run_blocking_ipc(move || portal::resolve_send_device(&sock, device_id)).await;
    let paths = match portal::pick_files_via_portal().await {
        Ok(paths) => paths,
        Err(err) => exit_with_error(err),
    };
    if paths.is_empty() {
        return;
    }
    handle_ipc_request(IpcRequest::SendFiles {
        device_id: target,
        paths,
    })
    .await;
}

async fn dispatch_action(action: CliAction) {
    match action {
        CliAction::Version => println!("keynear {}", env!("CARGO_PKG_VERSION")),
        CliAction::Help => println!("{CLI_USAGE_TEXT}"),
        CliAction::Run { config_path } => run_daemon(config_path).await,
        CliAction::Settings {
            config_path,
            validate_only,
        } => {
            let sock = daemon_socket_path();
            let output = run_blocking_ipc(move || {
                run_settings_command(&sock, config_path.as_ref(), validate_only)
            })
            .await;
            println!("{output}");
        }
        CliAction::Send { device_id, paths } => {
            let sock = daemon_socket_path();
            let target =
                run_blocking_ipc(move || portal::resolve_send_device(&sock, device_id)).await;
            handle_ipc_request(IpcRequest::SendFiles {
                device_id: target,
                paths,
            })
            .await;
        }
        CliAction::SendPicker { device_id } => run_send_picker(device_id).await,
        _ => unreachable!(),
    }
}
