use keynear_core::CallActionKind;
use tracing::info;

pub fn log_call_action(call_id: &str, action: CallActionKind) {
    let name = match action {
        CallActionKind::Answer => "answer",
        CallActionKind::Decline => "decline",
        CallActionKind::Mute => "mute",
    };
    info!(event = "carina_call_action", call_id = %call_id, action = name);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_actions_map_to_stable_log_names() {
        log_call_action("call-1", CallActionKind::Answer);
        log_call_action("call-1", CallActionKind::Decline);
        log_call_action("call-1", CallActionKind::Mute);
    }
}
