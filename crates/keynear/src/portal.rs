use std::path::{Path, PathBuf};

use keynear_ipc::{send_request_to, IpcRequest, IpcResponse};
use keynear_protocol::{sanitize_device_id, KeynearError};

fn validation(field: &'static str, message: String) -> KeynearError {
    KeynearError::Validation { field, message }
}

fn local_path_part(uri: &str) -> Result<String, KeynearError> {
    let after_scheme = uri
        .strip_prefix("file://")
        .ok_or_else(|| validation("uri", format!("expected file:// URI, got '{uri}'")))?;
    if let Some(rest) = after_scheme.strip_prefix("localhost/") {
        Ok(format!("/{rest}"))
    } else if after_scheme.starts_with('/') {
        Ok(after_scheme.to_string())
    } else {
        Err(validation(
            "uri",
            format!("unsupported non-local file URI '{uri}'"),
        ))
    }
}

fn decode_hex_byte(hi: u8, lo: u8) -> Option<u8> {
    let h = (hi as char).to_digit(16)?;
    let l = (lo as char).to_digit(16)?;
    Some(((h << 4) | l) as u8)
}

fn percent_decode(bytes: &[u8]) -> Result<Vec<u8>, KeynearError> {
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(validation(
                    "uri",
                    "truncated percent-encoding in file URI".to_string(),
                ));
            }
            let byte = decode_hex_byte(bytes[i + 1], bytes[i + 2])
                .ok_or_else(|| validation("uri", "invalid hex escape in file URI".to_string()))?;
            decoded.push(byte);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    Ok(decoded)
}

pub fn file_uri_to_path(uri: &str) -> Result<PathBuf, KeynearError> {
    let raw_path = local_path_part(uri)?;
    let decoded = percent_decode(raw_path.as_bytes())?;

    if decoded.is_empty() || decoded.contains(&0) {
        return Err(validation(
            "uri",
            "file URI resolves to an empty or NUL path".to_string(),
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStringExt;
        Ok(PathBuf::from(std::ffi::OsString::from_vec(decoded)))
    }
    #[cfg(not(unix))]
    {
        String::from_utf8(decoded)
            .map(PathBuf::from)
            .map_err(|_| validation("uri", "file URI is not valid UTF-8".to_string()))
    }
}

pub async fn pick_paths_via_portal(directory: bool) -> Result<Vec<PathBuf>, KeynearError> {
    let title = if directory {
        "Send folder via Keynear"
    } else {
        "Send files via Keynear"
    };
    let pending = ashpd::desktop::file_chooser::SelectedFiles::open_file()
        .title(title)
        .modal(true)
        .directory(directory)
        .multiple(true)
        .send()
        .await
        .map_err(|err| KeynearError::Transport(format!("file chooser portal failed: {err}")))?;
    match pending.response() {
        Ok(response) => response
            .uris()
            .iter()
            .map(|uri| file_uri_to_path(uri.as_str()))
            .collect(),
        Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => Ok(Vec::new()),
        Err(err) => Err(KeynearError::Transport(format!(
            "file chooser portal failed: {err}"
        ))),
    }
}

pub fn resolve_send_device(
    socket_path: &Path,
    explicit_device: Option<String>,
) -> Result<String, KeynearError> {
    if let Some(id) = explicit_device {
        return sanitize_device_id(&id);
    }
    match send_request_to(socket_path, &IpcRequest::ListDevices)? {
        IpcResponse::Devices(devices) => match devices.as_slice() {
            [] => Err(validation(
                "deviceId",
                "no paired devices found; pair a device before sending files".to_string(),
            )),
            [only] => Ok(only.device_id.clone()),
            many => {
                let ids: Vec<&str> = many.iter().map(|d| d.device_id.as_str()).collect();
                Err(validation(
                    "deviceId",
                    format!(
                        "multiple paired devices found ({}); pass --device <device_id>",
                        ids.join(", ")
                    ),
                ))
            }
        },
        IpcResponse::Error { message } => Err(KeynearError::Transfer(message)),
        other => Err(KeynearError::Protocol(format!(
            "unexpected IPC response while listing paired devices: {other:?}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::os::unix::net::UnixListener;
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use keynear_ipc::{
        decode_request, encode_response, prepare_daemon_socket_dir, set_daemon_socket_permissions,
    };
    use keynear_protocol::{DeviceRole, PairedDevice};

    use super::*;

    fn sample_device(id: &str) -> PairedDevice {
        PairedDevice {
            device_id: id.to_string(),
            display_name: id.to_string(),
            device_role: DeviceRole::Mobile,
            static_x25519_pubkey: "ab".repeat(32),
            last_ble_address: String::new(),
            last_lan_address: String::new(),
            last_endpoint_id: String::new(),
            capability_flags: 1,
            auto_accept_files: false,
            paired_at: "1".to_string(),
        }
    }

    fn spawn_mock_devices_server(
        label: &str,
        devices: Vec<PairedDevice>,
    ) -> (PathBuf, PathBuf, thread::JoinHandle<()>) {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("keynear-portal-{label}-{nanos}"));
        let sock_path = dir.join("daemon.sock");
        prepare_daemon_socket_dir(&sock_path).expect("prepare socket dir");
        let listener = UnixListener::bind(&sock_path).expect("bind socket");
        set_daemon_socket_permissions(&sock_path).expect("set socket perms");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut len_buf = [0u8; 4];
            stream.read_exact(&mut len_buf).expect("read len");
            let mut req_buf = vec![0u8; u32::from_be_bytes(len_buf) as usize];
            stream.read_exact(&mut req_buf).expect("read body");
            assert_eq!(
                decode_request(&req_buf).expect("decode"),
                IpcRequest::ListDevices
            );
            let encoded = encode_response(&IpcResponse::Devices(devices)).expect("encode");
            stream
                .write_all(&(encoded.len() as u32).to_be_bytes())
                .expect("write len");
            stream.write_all(&encoded).expect("write body");
        });
        (dir, sock_path, handle)
    }

    #[test]
    fn file_uri_to_path_decodes_escapes_and_rejects_invalid_uris() {
        assert_eq!(
            file_uri_to_path("file:///tmp/my%20file%25.txt").unwrap(),
            PathBuf::from("/tmp/my file%.txt")
        );
        assert_eq!(
            file_uri_to_path("file://localhost/tmp/-leading.pdf").unwrap(),
            PathBuf::from("/tmp/-leading.pdf")
        );
        assert!(file_uri_to_path("https://example.com/a").is_err());
        assert!(file_uri_to_path("file://remote/tmp/a").is_err());
        assert!(file_uri_to_path("file:///tmp/bad%2").is_err());
        assert!(file_uri_to_path("file:///tmp/bad%ZZ").is_err());
        assert!(file_uri_to_path("file:///tmp/nul%00byte").is_err());
        assert!(file_uri_to_path("file://").is_err());
    }

    #[test]
    fn resolve_send_device_handles_explicit_zero_one_and_multiple_devices() {
        assert_eq!(
            resolve_send_device(Path::new("/nonexistent.sock"), Some("pixel-9".to_string()))
                .unwrap(),
            "pixel-9"
        );

        let (dir0, sock0, h0) = spawn_mock_devices_server("zero", Vec::new());
        let err0 = resolve_send_device(&sock0, None).unwrap_err();
        assert!(err0.to_string().contains("no paired devices found"));
        h0.join().expect("server 0");
        let _ = fs::remove_dir_all(&dir0);

        let (dir1, sock1, h1) = spawn_mock_devices_server("one", vec![sample_device("pixel-9")]);
        assert_eq!(resolve_send_device(&sock1, None).unwrap(), "pixel-9");
        h1.join().expect("server 1");
        let _ = fs::remove_dir_all(&dir1);

        let (dir2, sock2, h2) = spawn_mock_devices_server(
            "many",
            vec![sample_device("pixel-9"), sample_device("tablet-1")],
        );
        let err2 = resolve_send_device(&sock2, None).unwrap_err().to_string();
        assert!(err2.contains("pixel-9") && err2.contains("tablet-1") && err2.contains("--device"));
        h2.join().expect("server 2");
        let _ = fs::remove_dir_all(&dir2);
    }
}
