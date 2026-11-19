use gtk4::prelude::*;
use keynear_protocol::{DaemonConfig, VisibilityMode, EVERYONE_NEARBY_TIMEOUT_SECS};

use super::{format_visibility_label, ActiveTab, SettingsInit, SettingsModel, SubScreen};
use crate::portal::{item_from_path, persist_config, refresh_from_daemon, sync_visibility};

impl SettingsModel {
    pub fn from_init(init: SettingsInit) -> Self {
        let remaining = if init.config.visibility_mode == VisibilityMode::EveryoneNearby {
            init.everyone_remaining_secs
                .unwrap_or(EVERYONE_NEARBY_TIMEOUT_SECS)
        } else {
            0
        };
        let selected_send_device_id = init
            .selected_send_device_id
            .or_else(|| init.paired_devices.first().map(|dev| dev.device_id.clone()));
        let initial_tab = if init.send_picker_active || !init.send_paths.is_empty() {
            ActiveTab::Send
        } else {
            ActiveTab::Receive
        };
        let mut queue = Vec::with_capacity(init.send_paths.len());
        let mut seq = 1u64;
        for path in &init.send_paths {
            queue.push(item_from_path(seq, path.clone()));
            seq += 1;
        }
        Self {
            visibility_mode: init.config.visibility_mode,
            everyone_remaining_secs: remaining,
            incoming_directory: init.config.incoming_directory.clone(),
            config: init.config,
            config_path: init.config_path,
            error_message: init.error_message,
            warning_message: init.warning_message,
            transfer_error: init.transfer_error,
            send_error: None,
            nearby_peers: init.nearby_peers,
            paired_devices: init.paired_devices,
            transfers: init.transfers,
            send_paths: init.send_paths,
            send_queue: queue,
            next_queue_seq: seq,
            selected_send_device_id,
            sending_in_flight: false,
            active_tab: initial_tab,
            sub_screen: None,
            sheet_expanded: false,
            text_composer_open: false,
            text_draft: String::new(),
            mime_editor_open: false,
            new_mime_input: String::new(),
            dark_theme: if gtk4::is_initialized() {
                gtk4::Settings::default()
                    .map(|s| s.is_gtk_application_prefer_dark_theme())
                    .unwrap_or(false)
            } else {
                false
            },
            snackbar_message: None,
            snackbar_ttl_secs: 0,
            renaming_device_id: None,
            rename_draft: String::new(),
            pending_pairing: None,
            pending_unpair_id: None,
            device_store: init.device_store,
            ipc_socket_path: init.ipc_socket_path,
            root_window: None,
        }
    }

    pub fn visibility_mode(&self) -> VisibilityMode {
        self.visibility_mode
    }

    pub fn visibility_status_text(&self) -> String {
        format_visibility_label(self.visibility_mode, self.everyone_remaining_secs)
    }

    pub fn auto_accept_from_paired(&self) -> bool {
        self.config.auto_accept_from_paired
    }

    pub fn incoming_directory(&self) -> &str {
        &self.incoming_directory
    }

    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    pub fn active_tab(&self) -> ActiveTab {
        self.active_tab
    }

    pub fn sub_screen(&self) -> Option<&SubScreen> {
        self.sub_screen.as_ref()
    }

    pub fn ignored_mime_types(&self) -> &[String] {
        &self.config.ignored_mime_types
    }

    pub fn dark_theme(&self) -> bool {
        self.dark_theme
    }

    pub(crate) fn show_snackbar(&mut self, message: impl Into<String>) {
        self.snackbar_message = Some(message.into());
        self.snackbar_ttl_secs = 3;
    }

    pub(super) fn apply_visibility_mode(&mut self, mode: VisibilityMode) {
        self.visibility_mode = mode;
        self.config.visibility_mode = mode;
        self.everyone_remaining_secs = if mode == VisibilityMode::EveryoneNearby {
            EVERYONE_NEARBY_TIMEOUT_SECS
        } else {
            0
        };
        sync_visibility(self, mode);
        self.show_snackbar(match self.visibility_mode {
            VisibilityMode::Hidden => "Visibility set to hidden.",
            VisibilityMode::PairedOnly => "Listening for paired computers.",
            VisibilityMode::EveryoneNearby => "Visible to everyone nearby for 10 minutes.",
        });
    }

    pub(super) fn handle_tick(&mut self) {
        if self.visibility_mode == VisibilityMode::EveryoneNearby {
            if self.everyone_remaining_secs <= 1 {
                self.visibility_mode = VisibilityMode::PairedOnly;
                self.everyone_remaining_secs = 0;
                self.show_snackbar("Everyone nearby expired. Reverted to Paired only.");
            } else {
                self.everyone_remaining_secs -= 1;
            }
        }
        if let Some(ref mut prompt) = self.pending_pairing {
            if prompt.remaining_secs <= 1 {
                self.pending_pairing = None;
                self.error_message =
                    Some("Pairing verification timed out after 120 seconds.".to_string());
            } else {
                prompt.remaining_secs -= 1;
            }
        }
        if self.snackbar_ttl_secs > 0 {
            self.snackbar_ttl_secs -= 1;
            if self.snackbar_ttl_secs == 0 {
                self.snackbar_message = None;
            }
        }
        refresh_from_daemon(self, false);
    }

    pub(super) fn set_config_flag(
        &mut self,
        enabled: bool,
        select: impl FnOnce(&mut DaemonConfig) -> &mut bool,
    ) {
        let slot = select(&mut self.config);
        if *slot != enabled {
            *slot = enabled;
            self.error_message = None;
            persist_config(self);
        }
    }

    pub(super) fn toggle_dark_theme(&mut self, enabled: bool) {
        if self.dark_theme == enabled {
            return;
        }
        self.dark_theme = enabled;
        if let Some(ref win) = self.root_window {
            if enabled {
                win.add_css_class("dark");
            } else {
                win.remove_css_class("dark");
            }
        }
    }
}
