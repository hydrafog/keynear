pub mod handlers;
pub mod server;

#[cfg(test)]
pub(crate) use handlers::{build_daemon_status, dispatch_ipc_request};
pub(crate) use handlers::{
    build_tray_state, handle_notification_action, handle_tray_command, IpcDispatchContext,
};
