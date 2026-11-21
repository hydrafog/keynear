use keynear_core::{
    select_fallback, FallbackChoice, FallbackMedia, FallbackProbeResult, FallbackStatus,
    KeynearError,
};
use zbus::fdo::DBusProxy;

const WPA_SUPPLICANT_SERVICE: &str = "fi.w1.wpa_supplicant1";

fn unavailable(media: FallbackMedia) -> FallbackProbeResult {
    FallbackProbeResult {
        media,
        status: FallbackStatus::Unavailable,
    }
}

pub async fn probe_wpa_supplicant() -> Result<FallbackProbeResult, KeynearError> {
    let connection = zbus::Connection::system()
        .await
        .map_err(|err| KeynearError::Transport(format!("vega system bus: {err}")))?;
    let proxy = DBusProxy::new(&connection)
        .await
        .map_err(|err| KeynearError::Transport(format!("vega dbus proxy: {err}")))?;
    let names = proxy
        .list_names()
        .await
        .map_err(|err| KeynearError::Transport(format!("vega list names: {err}")))?;
    let status = if names.iter().any(|name| *name == WPA_SUPPLICANT_SERVICE) {
        FallbackStatus::Available
    } else {
        FallbackStatus::Unavailable
    };
    Ok(FallbackProbeResult {
        media: FallbackMedia::WifiDirectR2,
        status,
    })
}

pub async fn probe_no_lan() -> Result<FallbackChoice, KeynearError> {
    let aware = unavailable(FallbackMedia::WifiAwareNan);
    let direct = match probe_wpa_supplicant().await {
        Ok(result) => result,
        Err(err) => {
            tracing::warn!(event = "vega_wpa_probe_unavailable", error = %err);
            unavailable(FallbackMedia::WifiDirectR2)
        }
    };
    Ok(select_fallback(aware, direct))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn no_lan_probe_never_fails_when_system_bus_is_absent() {
        let choice = probe_no_lan()
            .await
            .expect("probe must degrade to a typed result");
        assert!(matches!(
            choice.media,
            FallbackMedia::WifiDirectR2 | FallbackMedia::None
        ));
    }
}
