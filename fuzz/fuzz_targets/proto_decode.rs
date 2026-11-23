#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let capped = &data[..data.len().min(keynear_protocol::MAX_CONTROL_FRAME_BYTES)];
    let _ = keynear_protocol::decode_envelope(capped);
    let _ = keynear_protocol::decode_control_frame(capped);
});
