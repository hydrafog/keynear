use keynear_core::{AudioSinkProfile, KeynearError};
use zbus::fdo::ObjectManagerProxy;

use super::SinkProfileObservation;

const MPRIS_PREFIX: &str = "org.mpris.MediaPlayer2.";
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const MPRIS_PLAYER: &str = "org.mpris.MediaPlayer2.Player";
const BLUEZ_TRANSPORT: &str = "org.bluez.Transport1";
const BLUEZ_DEST: &str = "org.bluez";
const BLUEZ_ROOT: &str = "/";
const BLUEZ_PROFILE_IFACE: &str = "org.bluez.MediaTransport1";

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

async fn transport_path_for(profile: AudioSinkProfile) -> Result<Option<String>, KeynearError> {
    let connection = system_bus().await?;
    let proxy = ObjectManagerProxy::builder(&connection)
        .destination(BLUEZ_DEST)
        .path(BLUEZ_ROOT)
        .build()
        .await
        .map_err(backend_error)?;
    let objects = proxy.get_managed_objects().await.map_err(backend_error)?;
    for (path, interfaces) in objects {
        let matched = interfaces.iter().any(|(interface, properties)| {
            interface.as_str() == BLUEZ_TRANSPORT
                && properties.iter().any(|(key, value)| {
                    key.as_str() == "Profile"
                        && value
                            .downcast_ref::<&str>()
                            .ok()
                            .map(|name| AudioSinkProfile::parse(name) == Some(profile))
                            .unwrap_or(false)
                })
        });
        if matched {
            return Ok(Some(path.to_string()));
        }
    }
    Ok(None)
}

pub async fn switch_profile(profile: AudioSinkProfile, method: &str) -> Result<(), KeynearError> {
    let Some(path) = transport_path_for(profile).await? else {
        return Err(backend_error(format!(
            "no BlueZ transport exposes profile {}",
            profile.as_str()
        )));
    };
    let connection = system_bus().await?;
    connection
        .call_method(
            Some(BLUEZ_DEST),
            path.as_str(),
            Some(BLUEZ_PROFILE_IFACE),
            method,
            &(),
        )
        .await
        .map_err(backend_error)?;
    Ok(())
}
