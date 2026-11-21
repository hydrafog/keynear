use keynear_core::{AudioSinkProfile, KeynearError};
use zbus::fdo::ObjectManagerProxy;

use super::SinkProfileObservation;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const MPRIS_PLAYER: &str = "org.mpris.MediaPlayer2.Player";
const BLUEZ_TRANSPORT: &str = "org.bluez.Transport1";

fn backend_error(err: impl std::fmt::Display) -> KeynearError {
    KeynearError::Bluetooth(format!("vega dbus backend: {err}"))
}

async fn session_bus() -> Result<zbus::Connection, KeynearError> {
    zbus::Connection::session().await.map_err(backend_error)
}

async fn system_bus() -> Result<zbus::Connection, KeynearError> {
    zbus::Connection::system().await.map_err(backend_error)
}

async fn mpris_players() -> Result<Vec<String>, KeynearError> {
    let connection = session_bus().await?;
    let reply = connection
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "ListNames",
            &(),
        )
        .await
        .map_err(backend_error)?;
    let names: Vec<String> = reply.body().deserialize().map_err(backend_error)?;
    Ok(names
        .into_iter()
        .filter(|name| name.starts_with(MPRIS_PREFIX))
        .collect())
}

pub async fn call_players(method: &str) -> Result<usize, KeynearError> {
    let players = mpris_players().await?;
    let connection = session_bus().await?;
    let mut delivered = 0usize;
    for player in players {
        let result = connection
            .call_method(
                Some(player.as_str()),
                MPRIS_PATH,
                Some(MPRIS_PLAYER),
                method,
                &(),
            )
            .await;
        if result.is_ok() {
            delivered += 1;
        }
    }
    Ok(delivered)
}

pub async fn observe_sink_profile() -> Result<SinkProfileObservation, KeynearError> {
    let connection = system_bus().await?;
    let proxy = ObjectManagerProxy::builder(&connection)
        .destination("org.bluez")
        .path("/")
        .build()
        .await
        .map_err(backend_error)?;
    let objects = proxy.get_managed_objects().await.map_err(backend_error)?;
    for (path, interfaces) in objects {
        for (interface, properties) in interfaces {
            if interface.as_str() != BLUEZ_TRANSPORT {
                continue;
            }
            for (key, value) in properties {
                if key.as_str() != "Profile" {
                    continue;
                }
                if let Ok(name) = value.downcast_ref::<&str>() {
                    if let Some(profile) = AudioSinkProfile::parse(name) {
                        return Ok(SinkProfileObservation {
                            object_path: path.to_string(),
                            profile_name: (*name).to_string(),
                            profile,
                        });
                    }
                }
            }
        }
    }
    Ok(SinkProfileObservation::none())
}
