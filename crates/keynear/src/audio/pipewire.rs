use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Once;

use keynear_core::{AudioSinkProfile, KeynearError};
use pipewire as pw;

static PIPEWIRE_INIT: Once = Once::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SinkObservation {
    pub node_name: String,
    pub media_class: String,
    pub profile: Option<AudioSinkProfile>,
}

fn observer_error(err: impl std::fmt::Display) -> KeynearError {
    KeynearError::Bluetooth(format!("pipewire observer failed: {err}"))
}

pub fn observe_sinks() -> Result<Vec<SinkObservation>, KeynearError> {
    PIPEWIRE_INIT.call_once(pw::init);
    let mainloop = pw::main_loop::MainLoopRc::new(None).map_err(observer_error)?;
    let context = pw::context::ContextRc::new(&mainloop, None).map_err(observer_error)?;
    let core = context.connect_rc(None).map_err(observer_error)?;
    let registry = core.get_registry().map_err(observer_error)?;

    let sinks = Rc::new(RefCell::new(Vec::new()));
    let done = Rc::new(Cell::new(false));

    let sinks_clone = sinks.clone();
    let registry_listener = registry
        .add_listener_local()
        .global(move |global| {
            let Some(props) = global.props else {
                return;
            };
            let Some(media_class) = props.get("media.class") else {
                return;
            };
            if media_class != "Audio/Sink" {
                return;
            }
            sinks_clone.borrow_mut().push(SinkObservation {
                node_name: props.get("node.name").unwrap_or("unknown").to_string(),
                media_class: media_class.to_string(),
                profile: props
                    .get("api.bluez5.profile")
                    .and_then(AudioSinkProfile::parse),
            });
        })
        .register();

    let mainloop_clone = mainloop.clone();
    let done_clone = done.clone();
    let pending = core.sync(0).map_err(observer_error)?;
    let core_listener = core
        .add_listener_local()
        .done(move |id, seq| {
            if id == pw::core::PW_ID_CORE && seq == pending {
                done_clone.set(true);
                mainloop_clone.quit();
            }
        })
        .register();

    while !done.get() {
        mainloop.run();
    }
    drop(core_listener);
    drop(registry_listener);

    let observations = sinks.borrow().clone();
    Ok(observations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sink_observation_keeps_profile_from_bluez_property() {
        let sink = SinkObservation {
            node_name: "bluez_output".to_string(),
            media_class: "Audio/Sink".to_string(),
            profile: AudioSinkProfile::parse("bap_sink"),
        };
        assert_eq!(sink.profile, Some(AudioSinkProfile::BapSink));
        assert_eq!(sink.media_class, "Audio/Sink");
    }
}
