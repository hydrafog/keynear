#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let capped = &data[..data.len().min(keynear_ipc::MAX_IPC_FRAME_BYTES)];
    if let Ok(request) = keynear_ipc::decode_request(capped) {
        let encoded = keynear_ipc::encode_request(&request).expect("re-encode request");
        assert_eq!(
            keynear_ipc::decode_request(&encoded).expect("re-decode request"),
            request
        );
    }
    if let Ok(response) = keynear_ipc::decode_response(capped) {
        let encoded = keynear_ipc::encode_response(&response).expect("re-encode response");
        assert_eq!(
            keynear_ipc::decode_response(&encoded).expect("re-decode response"),
            response
        );
    }
});
