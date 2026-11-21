use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use gtk4::glib;
use gtk4::prelude::*;
use relm4::ComponentSender;

use crate::state::{SendItemKind, SendQueueItem, SettingsModel, SettingsMsg};

use super::uri::file_uri_to_path;

static PORTAL_TOKIO_RT: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

fn portal_runtime() -> Result<&'static tokio::runtime::Runtime, String> {
    let entry = PORTAL_TOKIO_RT.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("keynear-portal")
            .enable_all()
            .build()
            .map_err(|e| format!("failed to initialize portal runtime: {e}"))
    });
    match entry {
        Ok(rt) => Ok(rt),
        Err(err) => Err(err.clone()),
    }
}

async fn query_portal(
    title: &'static str,
    multiple: bool,
    directory: bool,
    portal_id: Option<ashpd::WindowIdentifier>,
) -> Result<Vec<PathBuf>, String> {
    let rt = portal_runtime()?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    rt.spawn(async move {
        let res = async {
            let req = ashpd::desktop::file_chooser::SelectedFiles::open_file()
                .title(title)
                .modal(true)
                .multiple(multiple)
                .directory(directory)
                .identifier(portal_id);
            let selected = req.send().await.map_err(|e| e.to_string())?;
            match selected.response() {
                Ok(response) => response
                    .uris()
                    .iter()
                    .map(|uri| file_uri_to_path(uri.as_str()).map_err(|e| e.to_string()))
                    .collect::<Result<Vec<PathBuf>, String>>(),
                Err(ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)) => {
                    Ok(Vec::new())
                }
                Err(err) => Err(err.to_string()),
            }
        }
        .await;
        let _ = tx.send(res);
    });
    rx.await
        .map_err(|_| "File chooser portal task did not complete.".to_string())?
}

async fn pick_paths_via_portal(
    window: &gtk4::Window,
    title: &'static str,
    multiple: bool,
    directory: bool,
) -> Result<Vec<PathBuf>, String> {
    let keep_alive = match window.native() {
        Some(native) => ashpd::WindowIdentifier::from_native(&native).await,
        None => None,
    };
    let portal_id = keep_alive
        .as_ref()
        .and_then(|id| id.to_string().parse::<ashpd::WindowIdentifierType>().ok())
        .map(ashpd::WindowIdentifier::from);
    let result = query_portal(title, multiple, directory, portal_id).await;
    drop(keep_alive);
    result
}

pub struct PortalPickerParams<'a, F> {
    pub window: Option<&'a gtk4::Window>,
    pub sender: &'a ComponentSender<SettingsModel>,
    pub title: &'static str,
    pub multiple: bool,
    pub directory: bool,
    pub on_paths: F,
}

pub(crate) fn spawn_portal_picker<F>(params: PortalPickerParams<'_, F>)
where
    F: Fn(Vec<PathBuf>) -> SettingsMsg + 'static,
{
    let PortalPickerParams {
        window,
        sender,
        title,
        multiple,
        directory,
        on_paths,
    } = params;
    let Some(win) = window.cloned() else {
        sender.input(SettingsMsg::PortalError(
            "Settings window is not available for file chooser.".to_string(),
        ));
        return;
    };
    let s = sender.clone();
    glib::spawn_future_local(async move {
        match pick_paths_via_portal(&win, title, multiple, directory).await {
            Ok(paths) if !paths.is_empty() => s.input(on_paths(paths)),
            Ok(_) => {}
            Err(err) => s.input(SettingsMsg::PortalError(err)),
        }
    });
}

pub(crate) fn estimate_path_bytes(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|m| if m.is_dir() { 4096 } else { m.len() })
        .unwrap_or(0)
}

pub(crate) fn item_from_path(seq: u64, path: PathBuf) -> SendQueueItem {
    let is_dir = std::fs::metadata(&path)
        .map(|m| m.is_dir())
        .unwrap_or(false);
    let raw_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());
    let display = if is_dir && !raw_name.ends_with('/') {
        format!("{raw_name}/")
    } else {
        raw_name
    };
    let size_bytes = estimate_path_bytes(&path);
    SendQueueItem {
        id: format!("path-{seq}"),
        name: display,
        size_bytes,
        kind: if is_dir {
            SendItemKind::Folder
        } else {
            SendItemKind::File
        },
        source_path: Some(path),
        text_payload: None,
    }
}

fn staging_dir(temp_dir: &mut Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(ref d) = temp_dir {
        return Ok(d.clone());
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let d = std::env::temp_dir().join(format!("keynear-gtk-send-{nonce}"));
    std::fs::create_dir_all(&d).map_err(|e| format!("Unable to prepare send staging: {e}"))?;
    *temp_dir = Some(d.clone());
    Ok(d)
}

fn sanitized_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

pub(crate) fn materialize_queue_paths(
    send_queue: &[SendQueueItem],
    send_paths: &[PathBuf],
) -> Result<(Vec<PathBuf>, Option<PathBuf>), String> {
    let mut paths = Vec::new();
    let mut temp_dir: Option<PathBuf> = None;

    for (idx, item) in send_queue.iter().enumerate() {
        if let Some(ref p) = item.source_path {
            paths.push(p.clone());
        } else if let Some(ref body) = item.text_payload {
            let dir = staging_dir(&mut temp_dir)?;
            let dest = dir.join(format!("{idx}_{}", sanitized_name(&item.name)));
            std::fs::write(&dest, body.as_bytes())
                .map_err(|e| format!("Unable to stage text item: {e}"))?;
            paths.push(dest);
        }
    }
    if paths.is_empty() && !send_paths.is_empty() {
        paths.extend(send_paths.iter().cloned());
    }
    Ok((paths, temp_dir))
}
