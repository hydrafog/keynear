use std::path::PathBuf;
use std::time::Duration;

use keynear_ipc::{
    decode_request, encode_response, prepare_daemon_socket_dir, set_daemon_socket_permissions,
    IpcResponse, MAX_IPC_FRAME_BYTES,
};
use keynear_protocol::KeynearError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{info, warn};

use super::handlers::{dispatch_ipc_request, IpcDispatchContext};

const READ_TIMEOUT: Duration = Duration::from_secs(300);
const WRITE_TIMEOUT: Duration = Duration::from_secs(10);

async fn read_exact_timed(stream: &mut UnixStream, buf: &mut [u8]) -> Result<(), KeynearError> {
    timeout(READ_TIMEOUT, stream.read_exact(buf))
        .await
        .map_err(|_| KeynearError::Transport("IPC read timed out".to_string()))?
        .map_err(KeynearError::Io)
        .map(|_| ())
}

async fn write_all_timed(stream: &mut UnixStream, bytes: &[u8]) -> Result<(), KeynearError> {
    timeout(WRITE_TIMEOUT, stream.write_all(bytes))
        .await
        .map_err(|_| KeynearError::Transport("IPC write timed out".to_string()))?
        .map_err(KeynearError::Io)
}

pub(crate) fn bind_ipc_listener(
    sock_path: PathBuf,
) -> Result<(PathBuf, UnixListener), KeynearError> {
    prepare_daemon_socket_dir(&sock_path)?;
    let listener = UnixListener::bind(&sock_path)?;
    set_daemon_socket_permissions(&sock_path)?;
    info!(event = "ipc_listening", socket = %sock_path.display());
    Ok((sock_path, listener))
}

pub(crate) fn serve_ipc_listener(
    listener: UnixListener,
    ctx: IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) {
    tasks.push(tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let req_ctx = ctx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = handle_ipc_connection(stream, req_ctx).await {
                            warn!(event = "ipc_request_failed", error = %err);
                        }
                    });
                }
                Err(err) => {
                    warn!(event = "ipc_accept_failed", error = %err);
                    break;
                }
            }
        }
    }));
}

#[allow(dead_code)]
pub(crate) async fn bind_and_serve_ipc(
    sock_path: PathBuf,
    ctx: IpcDispatchContext,
    tasks: &mut Vec<JoinHandle<()>>,
) -> Result<PathBuf, KeynearError> {
    let (bound_path, listener) = bind_ipc_listener(sock_path)?;
    serve_ipc_listener(listener, ctx, tasks);
    Ok(bound_path)
}

async fn read_frame_len(stream: &mut UnixStream) -> Result<Option<usize>, KeynearError> {
    let mut len_buf = [0u8; 4];
    let first = timeout(READ_TIMEOUT, stream.read(&mut len_buf))
        .await
        .map_err(|_| KeynearError::Transport("IPC read timed out".to_string()))?
        .map_err(KeynearError::Io)?;
    if first == 0 {
        return Ok(None);
    }
    if first < 4 {
        read_exact_timed(stream, &mut len_buf[first..]).await?;
    }
    Ok(Some(u32::from_be_bytes(len_buf) as usize))
}

pub(crate) async fn handle_ipc_connection(
    mut stream: UnixStream,
    ctx: IpcDispatchContext,
) -> Result<(), KeynearError> {
    let Some(frame_len) = read_frame_len(&mut stream).await? else {
        return Ok(());
    };
    if frame_len == 0 {
        return Err(KeynearError::Protocol(
            "IPC request frame length must be positive".to_string(),
        ));
    }
    if frame_len > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: frame_len,
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    let mut payload = vec![0u8; frame_len];
    read_exact_timed(&mut stream, &mut payload).await?;

    let response = match decode_request(&payload) {
        Ok(req) => dispatch_ipc_request(req, &ctx).await,
        Err(err) => IpcResponse::Error {
            message: err.to_string(),
        },
    };

    let encoded = encode_response(&response)?;
    let resp_len = u32::try_from(encoded.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: encoded.len(),
        max: MAX_IPC_FRAME_BYTES,
    })?;
    write_all_timed(&mut stream, &resp_len.to_be_bytes()).await?;
    write_all_timed(&mut stream, &encoded).await?;
    timeout(WRITE_TIMEOUT, stream.flush())
        .await
        .map_err(|_| KeynearError::Transport("IPC flush timed out".to_string()))??;
    Ok(())
}
