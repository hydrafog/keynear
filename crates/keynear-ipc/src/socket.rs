use crate::config::*;
use crate::*;

use crate::request::encode_request;
use crate::response::decode_response;

pub fn daemon_socket_path() -> PathBuf {
    default_config_dir().join("daemon.sock")
}

pub fn prepare_daemon_socket_dir(socket_path: &Path) -> Result<(), KeynearError> {
    if let Some(parent) = socket_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
    }

    match fs::symlink_metadata(socket_path) {
        Ok(meta) => {
            if !meta.file_type().is_socket() {
                return Err(KeynearError::Config(format!(
                    "refusing to replace non-socket path at {}",
                    socket_path.display()
                )));
            }
            if UnixStream::connect(socket_path).is_ok() {
                return Err(KeynearError::Config(format!(
                    "keynear daemon is already running at {}",
                    socket_path.display()
                )));
            }
            fs::remove_file(socket_path)?;
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(KeynearError::Io(err)),
    }

    Ok(())
}

pub fn set_daemon_socket_permissions(socket_path: &Path) -> Result<(), KeynearError> {
    fs::set_permissions(socket_path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

pub fn send_request(request: &IpcRequest) -> Result<IpcResponse, KeynearError> {
    send_request_to(&daemon_socket_path(), request)
}

pub fn send_request_to(
    socket_path: &Path,
    request: &IpcRequest,
) -> Result<IpcResponse, KeynearError> {
    let encoded = encode_request(request)?;
    let mut stream = UnixStream::connect(socket_path).map_err(|err| match err.kind() {
        std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused => {
            KeynearError::Transport(format!(
                "keynear daemon is not running (socket {} unavailable)",
                socket_path.display()
            ))
        }
        _ => KeynearError::Io(err),
    })?;

    let read_timeout = match request {
        IpcRequest::SendFiles { .. } => Duration::from_secs(IPC_SEND_TIMEOUT_SECS),
        _ => Duration::from_secs(IPC_CONTROL_TIMEOUT_SECS),
    };
    stream.set_write_timeout(Some(Duration::from_secs(IPC_CONTROL_TIMEOUT_SECS)))?;
    stream.set_read_timeout(Some(read_timeout))?;

    let frame_len = u32::try_from(encoded.len()).map_err(|_| KeynearError::FrameTooLarge {
        size: encoded.len(),
        max: MAX_IPC_FRAME_BYTES,
    })?;
    stream.write_all(&frame_len.to_be_bytes())?;
    stream.write_all(&encoded)?;
    stream.flush()?;

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let resp_len = u32::from_be_bytes(len_buf) as usize;
    if resp_len == 0 {
        return Err(KeynearError::Protocol(
            "IPC response frame length must be positive".to_string(),
        ));
    }
    if resp_len > MAX_IPC_FRAME_BYTES {
        return Err(KeynearError::FrameTooLarge {
            size: resp_len,
            max: MAX_IPC_FRAME_BYTES,
        });
    }
    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf)?;
    decode_response(&resp_buf)
}
