use keynear_protocol::KeynearError;

const CLIPBOARD_BRIDGE_DEST: &str = "org.keynear.ClipboardBridge";
const CLIPBOARD_BRIDGE_PATH: &str = "/org/keynear/ClipboardBridge";
const CLIPBOARD_BRIDGE_IFACE: &str = "org.keynear.ClipboardBridge";

pub struct ClipboardBridgeProxy {
    proxy: zbus::Proxy<'static>,
}

impl ClipboardBridgeProxy {
    pub async fn connect() -> Option<Self> {
        let conn = zbus::Connection::session().await.ok()?;
        let proxy = zbus::Proxy::new(
            &conn,
            CLIPBOARD_BRIDGE_DEST,
            CLIPBOARD_BRIDGE_PATH,
            CLIPBOARD_BRIDGE_IFACE,
        )
        .await
        .ok()?;
        Some(Self { proxy })
    }

    pub async fn sync(&self) -> Result<(), KeynearError> {
        self.proxy
            .call_method("Sync", &())
            .await
            .map(|_| ())
            .map_err(|err| KeynearError::Transport(format!("clipboard bridge Sync failed: {err}")))
    }

    pub async fn status(&self) -> Result<String, KeynearError> {
        let reply = self.proxy.call_method("Status", &()).await.map_err(|err| {
            KeynearError::Transport(format!("clipboard bridge Status failed: {err}"))
        })?;
        reply.body()
            .deserialize::<String>()
            .map_err(|err| KeynearError::Protocol(format!("clipboard bridge Status invalid: {err}")))
    }
}
