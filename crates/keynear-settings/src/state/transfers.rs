use std::path::PathBuf;

use keynear_ipc::IpcRequest;
use keynear_protocol::TransferSession;

use super::SettingsModel;
use crate::portal::{item_from_path, send_transfer_action};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendItemKind {
    File,
    Folder,
    Text,
    Clipboard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendQueueItem {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub kind: SendItemKind,
    pub source_path: Option<PathBuf>,
    pub text_payload: Option<String>,
}

impl SettingsModel {
    pub fn transfers(&self) -> &[TransferSession] {
        &self.transfers
    }

    pub fn transfer_error(&self) -> Option<&str> {
        self.transfer_error.as_deref()
    }

    pub fn send_paths(&self) -> &[PathBuf] {
        &self.send_paths
    }

    pub fn send_queue(&self) -> &[SendQueueItem] {
        &self.send_queue
    }

    pub fn selected_send_device_id(&self) -> Option<&str> {
        self.selected_send_device_id.as_deref()
    }

    pub fn send_error(&self) -> Option<&str> {
        self.send_error.as_deref()
    }

    pub(super) fn stage_text_item(&mut self, text: &str, is_clipboard: bool) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return;
        }
        let seq = self.next_queue_seq;
        self.next_queue_seq += 1;
        let name = if is_clipboard {
            "clipboard-text.txt".to_string()
        } else {
            let preview: String = trimmed.chars().take(28).collect();
            format!("{preview}.txt")
        };
        self.send_queue.push(SendQueueItem {
            id: format!("text-{seq}"),
            name,
            size_bytes: trimmed.len() as u64,
            kind: if is_clipboard {
                SendItemKind::Clipboard
            } else {
                SendItemKind::Text
            },
            source_path: None,
            text_payload: Some(trimmed.to_string()),
        });
        self.send_error = None;
    }

    pub(super) fn handle_stage_send_paths(&mut self, paths: Vec<std::path::PathBuf>) {
        for path in paths {
            if !self.send_paths.contains(&path) {
                self.send_paths.push(path.clone());
            }
            let seq = self.next_queue_seq;
            self.next_queue_seq += 1;
            self.send_queue.push(item_from_path(seq, path));
        }
        self.send_error = None;
    }

    pub(super) fn handle_remove_queue_item(&mut self, id: &str) {
        if let Some(pos) = self.send_queue.iter().position(|item| item.id == id) {
            let removed = self.send_queue.remove(pos);
            if let Some(path) = removed.source_path {
                self.send_paths.retain(|p| p != &path);
            }
        }
    }

    pub(super) fn handle_send_completed(&mut self, result: Result<TransferSession, String>) {
        self.sending_in_flight = false;
        match result {
            Ok(session) => {
                if let Some(existing) = self
                    .transfers
                    .iter_mut()
                    .find(|t| t.transfer_id == session.transfer_id)
                {
                    *existing = session;
                } else {
                    self.transfers.insert(0, session);
                }
                self.send_paths.clear();
                self.send_queue.clear();
                self.send_error = None;
                self.transfer_error = None;
                self.show_snackbar("Transfer started.");
            }
            Err(err) => self.send_error = Some(err),
        }
    }

    pub(super) fn handle_accept_transfer(&mut self, transfer_id: String) {
        send_transfer_action(self, IpcRequest::AcceptTransfer { transfer_id });
    }

    pub(super) fn handle_decline_transfer(&mut self, transfer_id: String) {
        send_transfer_action(self, IpcRequest::DeclineTransfer { transfer_id });
    }

    pub(super) fn handle_cancel_transfer(&mut self, transfer_id: String) {
        send_transfer_action(self, IpcRequest::CancelTransfer { transfer_id });
    }
}
