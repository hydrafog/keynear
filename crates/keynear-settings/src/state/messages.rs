use crate::portal::{
    confirm_unpair_device, persist_config, refresh_from_daemon, set_device_auto_accept,
    submit_device_rename,
};

use super::{SettingsModel, SettingsMsg, SubScreen};

pub fn apply_msg(model: &mut SettingsModel, msg: SettingsMsg) {
    if apply_navigation(model, &msg) {
        return;
    }
    if apply_refresh_and_pairing(model, &msg) {
        return;
    }
    if apply_devices(model, &msg) {
        return;
    }
    if apply_config(model, &msg) {
        return;
    }
    if apply_send(model, &msg) {
        return;
    }
    apply_transfers(model, &msg);
}

fn apply_navigation(model: &mut SettingsModel, msg: &SettingsMsg) -> bool {
    match msg {
        SettingsMsg::SetVisibility(mode) => model.apply_visibility_mode(*mode),
        SettingsMsg::SwitchTab(tab) => {
            model.sub_screen = None;
            model.active_tab = *tab;
            model.pending_unpair_id = None;
        }
        SettingsMsg::NavigateBack => {
            model.sub_screen = None;
            model.pending_unpair_id = None;
        }
        SettingsMsg::ToggleSheetExpanded => model.sheet_expanded = !model.sheet_expanded,
        SettingsMsg::ToggleMimeExpanded => model.mime_editor_open = !model.mime_editor_open,
        SettingsMsg::UpdateMimeInput(raw) => model.new_mime_input = raw.clone(),
        SettingsMsg::ToggleDarkTheme(enabled) => model.toggle_dark_theme(*enabled),
        SettingsMsg::PortalError(err) => model.error_message = Some(err.clone()),
        _ => return false,
    }
    true
}

fn apply_refresh_and_pairing(model: &mut SettingsModel, msg: &SettingsMsg) -> bool {
    match msg {
        SettingsMsg::Tick => model.handle_tick(),
        SettingsMsg::RefreshPeers => {
            match model.device_store.load() {
                Ok(devices) => {
                    model.paired_devices = devices;
                    model.error_message = None;
                }
                Err(err) => model.error_message = Some(err.to_string()),
            }
            refresh_from_daemon(model, false);
        }
        SettingsMsg::RetryDaemon => {
            if let Ok(devices) = model.device_store.load() {
                model.paired_devices = devices;
            }
            refresh_from_daemon(model, true);
        }
        SettingsMsg::BeginPair(peer_token) => model.begin_pair(peer_token),
        SettingsMsg::ConfirmPairing => {
            model.pending_pairing = None;
            model.warning_message = Some(
                "Pairing approval queued; verify matching 6-digit code on the peer device."
                    .to_string(),
            );
            model.show_snackbar("Pairing approval queued.");
        }
        SettingsMsg::CancelPairing => {
            model.pending_pairing = None;
            model.show_snackbar("Pairing request rejected.");
        }
        _ => return false,
    }
    true
}

fn apply_devices(model: &mut SettingsModel, msg: &SettingsMsg) -> bool {
    match msg {
        SettingsMsg::OpenDeviceDetail(device_id) => {
            if let Some(dev) = model
                .paired_devices
                .iter()
                .find(|d| d.device_id == *device_id)
            {
                model.rename_draft = dev.display_name.clone();
                model.renaming_device_id = Some(dev.device_id.clone());
                model.pending_unpair_id = None;
                model.sub_screen = Some(SubScreen::DeviceDetail(dev.device_id.clone()));
            }
        }
        SettingsMsg::ResetBleBond(device_id) => {
            let name = model
                .paired_devices
                .iter()
                .find(|d| d.device_id == *device_id)
                .map(|d| d.display_name.clone())
                .unwrap_or_else(|| device_id.clone());
            model.show_snackbar(format!("Cleared Bluetooth bond for {name}."));
        }
        SettingsMsg::RequestUnpair(device_id) => {
            model.pending_unpair_id = Some(device_id.clone());
        }
        SettingsMsg::CancelUnpair => model.pending_unpair_id = None,
        SettingsMsg::ConfirmUnpair(device_id) => confirm_unpair_device(model, device_id),
        SettingsMsg::StartRename(device_id) => {
            if let Some(dev) = model
                .paired_devices
                .iter()
                .find(|d| d.device_id == *device_id)
            {
                model.renaming_device_id = Some(dev.device_id.clone());
                model.rename_draft = dev.display_name.clone();
                model.error_message = None;
            }
        }
        SettingsMsg::UpdateRenameDraft(text) => model.rename_draft = text.clone(),
        SettingsMsg::SubmitRename {
            device_id,
            display_name,
        } => submit_device_rename(model, device_id.clone(), display_name.clone()),
        SettingsMsg::SetAutoAcceptFiles { device_id, enabled } => {
            set_device_auto_accept(model, device_id.clone(), *enabled);
        }
        _ => return false,
    }
    true
}

fn apply_config(model: &mut SettingsModel, msg: &SettingsMsg) -> bool {
    match msg {
        SettingsMsg::SetGlobalAutoAccept(enabled) => {
            model.set_config_flag(*enabled, |c| &mut c.auto_accept_from_paired)
        }
        SettingsMsg::ToggleClipboardSync(enabled) => {
            model.set_config_flag(*enabled, |c| &mut c.clipboard_sync_enabled)
        }
        SettingsMsg::ToggleClipboardImages(enabled) => {
            model.set_config_flag(*enabled, |c| &mut c.clipboard_images_enabled)
        }
        SettingsMsg::ToggleOtpPassthrough(enabled) => {
            model.set_config_flag(*enabled, |c| &mut c.otp_passthrough_enabled)
        }
        SettingsMsg::ToggleAudioHandoff(enabled) => {
            model.set_config_flag(*enabled, |c| &mut c.audio_handoff_enabled)
        }
        SettingsMsg::ToggleNotifyAutoAccept(enabled) => {
            model.set_config_flag(*enabled, |c| &mut c.notify_auto_accept_complete)
        }
        SettingsMsg::AddIgnoredMime => add_ignored_mime(model),
        SettingsMsg::RemoveIgnoredMime(mime) => {
            let before = model.config.ignored_mime_types.len();
            model.config.ignored_mime_types.retain(|m| m != mime);
            if model.config.ignored_mime_types.len() != before {
                model.error_message = None;
                persist_config(model);
            }
        }
        SettingsMsg::SetIncomingDirectory(dir) => set_incoming_directory(model, dir),
        _ => return false,
    }
    true
}

fn add_ignored_mime(model: &mut SettingsModel) {
    const MAX_IPC_MIMES: usize = 128;
    let trimmed = model.new_mime_input.trim().to_string();
    if trimmed.is_empty() || model.config.ignored_mime_types.contains(&trimmed) {
        return;
    }
    if model.config.ignored_mime_types.len() >= MAX_IPC_MIMES {
        model.error_message = Some(format!(
            "Ignored MIME list cannot exceed {MAX_IPC_MIMES} entries."
        ));
        return;
    }
    model.config.ignored_mime_types.push(trimmed);
    model.new_mime_input.clear();
    model.error_message = None;
    persist_config(model);
}

fn set_incoming_directory(model: &mut SettingsModel, dir: &str) {
    let clean = dir.trim().to_string();
    if clean.is_empty() {
        model.error_message = Some("Incoming directory must not be empty.".to_string());
        return;
    }
    model.incoming_directory = clean.clone();
    model.config.incoming_directory = clean.clone();
    model.error_message = None;
    persist_config(model);
    model.show_snackbar(format!("Save folder set to {clean}."));
}

fn apply_send(model: &mut SettingsModel, msg: &SettingsMsg) -> bool {
    match msg {
        SettingsMsg::StageSendPaths(paths) => model.handle_stage_send_paths(paths.clone()),
        SettingsMsg::StageClipboardForSend => {
            model.stage_text_item("Clipboard text snapshot", true);
        }
        SettingsMsg::StageClipboardText(text) => {
            model.stage_text_item(text, true);
            model.show_snackbar("Clipboard text added to send queue.");
        }
        SettingsMsg::ToggleTextComposer => model.text_composer_open = !model.text_composer_open,
        SettingsMsg::UpdateTextDraft(text) => model.text_draft = text.clone(),
        SettingsMsg::StageTextSnippet => {
            let draft = model.text_draft.clone();
            if !draft.trim().is_empty() {
                model.stage_text_item(&draft, false);
                model.text_draft.clear();
                model.text_composer_open = false;
            }
        }
        SettingsMsg::RemoveSendQueueItem(id) => model.handle_remove_queue_item(id),
        SettingsMsg::ClearSendPaths => {
            model.send_paths.clear();
            model.send_queue.clear();
            model.send_error = None;
        }
        SettingsMsg::SelectSendDevice(device_id) => {
            model.selected_send_device_id = Some(device_id.clone());
            model.send_error = None;
        }
        SettingsMsg::SendCompleted(result) => model.handle_send_completed(result.clone()),
        SettingsMsg::PickSendFiles
        | SettingsMsg::PickSendFolder
        | SettingsMsg::PickIncomingDirectory
        | SettingsMsg::SendQueueToPeer(_) => {}
        _ => return false,
    }
    true
}

fn apply_transfers(model: &mut SettingsModel, msg: &SettingsMsg) -> bool {
    match msg {
        SettingsMsg::AcceptTransfer(transfer_id) => {
            model.handle_accept_transfer(transfer_id.clone())
        }
        SettingsMsg::DeclineTransfer(transfer_id) => {
            model.handle_decline_transfer(transfer_id.clone())
        }
        SettingsMsg::CancelTransfer(transfer_id) => {
            model.handle_cancel_transfer(transfer_id.clone())
        }
        _ => return false,
    }
    true
}
