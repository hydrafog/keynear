#![deny(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::too_many_arguments
)]

// TODO: drive VegaAudioController from the peer audio-control stream once its QUIC demux lands.
#[allow(dead_code)]
mod audio;
mod ble;
mod cli;
mod clipboard;
mod fallback;
mod fuse;
mod ipc;
mod keyring_oo7;
mod notifications;
mod perseus;
mod service;
mod tray;
mod vega;

use std::process;

use keynear_ipc::{daemon_socket_path, IpcRequest};
use tracing_subscriber::EnvFilter;

use crate::cli::{
    cli_action_to_ipc_request, load_daemon_config, parse_cli_args, run_ipc_command, CliAction,
    CLI_USAGE_TEXT,
};

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

    if let Some(request) = cli_action_to_ipc_request(&action) {
        handle_ipc_request(request).await;
        return;
    }

    dispatch_action(action).await;
}

async fn handle_ipc_request(request: IpcRequest) {
    let sock = daemon_socket_path();
    let output = match tokio::task::spawn_blocking(move || run_ipc_command(&sock, &request)).await {
        Ok(Ok(text)) => text,
        Ok(Err(err)) => {
            eprintln!("error: {err}");
            process::exit(1);
        }
        Err(err) => {
            eprintln!("error: command task failed: {err}");
            process::exit(1);
        }
    };
    println!("{output}");
}

async fn dispatch_action(action: CliAction) {
    match action {
        CliAction::Version => {
            println!("keynear-linux {}", env!("CARGO_PKG_VERSION"));
        }
        CliAction::Help => {
            println!("{CLI_USAGE_TEXT}");
        }
        CliAction::Run { config_path } => {
            let config = match load_daemon_config(config_path.as_ref()) {
                Ok(config) => config,
                Err(err) => {
                    eprintln!("error: {err}");
                    process::exit(1);
                }
            };
            if let Err(err) =
                service::DaemonRuntime::run_until_shutdown_with_config_path(&config, config_path)
                    .await
            {
                eprintln!("error: {err}");
                process::exit(1);
            }
        }
        CliAction::Send { .. }
        | CliAction::Accept { .. }
        | CliAction::Decline { .. }
        | CliAction::Cancel { .. }
        | CliAction::Transfers
        | CliAction::Status
        | CliAction::Devices
        | CliAction::SetVisibility { .. }
        | CliAction::RenameDevice { .. } => unreachable!(),
    }
}
