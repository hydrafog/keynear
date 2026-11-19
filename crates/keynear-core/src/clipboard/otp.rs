use keynear_protocol::KeynearError;

use super::history::ClipboardPolicy;

pub(crate) fn check_otp_policy(policy: &ClipboardPolicy, is_otp: bool) -> Result<(), KeynearError> {
    if is_otp && !policy.otp_passthrough_enabled {
        return Err(KeynearError::Validation {
            field: "otpPassthroughEnabled",
            message: "OTP pass-through is disabled".to_string(),
        });
    }
    Ok(())
}
