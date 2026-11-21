use super::types::{TransferEntry, MAX_ACTIVE_TRANSFERS, MAX_HISTORY_TRANSFERS};
use super::*;

pub(super) fn ensure_capacity(
    entries: &mut HashMap<String, TransferEntry>,
) -> Result<(), KeynearError> {
    let active_count = entries
        .values()
        .filter(|e| {
            matches!(
                e.session.state,
                TransferState::PendingConsent | TransferState::Active
            )
        })
        .count();
    if active_count >= MAX_ACTIVE_TRANSFERS {
        return Err(KeynearError::Transfer(
            "maximum active transfers limit reached".to_string(),
        ));
    }
    if entries.len() >= MAX_HISTORY_TRANSFERS {
        let removable: Vec<String> = entries
            .iter()
            .filter(|(_, e)| {
                !matches!(
                    e.session.state,
                    TransferState::PendingConsent | TransferState::Active
                )
            })
            .map(|(k, _)| k.clone())
            .take(entries.len() - MAX_HISTORY_TRANSFERS + 1)
            .collect();
        for key in removable {
            entries.remove(&key);
        }
    }
    Ok(())
}

pub(crate) fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(super) fn now_timestamp_string() -> String {
    format!("{}", now_millis())
}
