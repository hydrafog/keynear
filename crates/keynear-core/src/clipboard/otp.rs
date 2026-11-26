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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otp_passthrough_policy_allows_only_the_enabled_path() {
        let policy = ClipboardPolicy {
            otp_passthrough_enabled: true,
            ..ClipboardPolicy::default()
        };
        assert!(check_otp_policy(&policy, true).is_ok());
        assert!(check_otp_policy(&policy, false).is_ok());
        let disabled = ClipboardPolicy {
            otp_passthrough_enabled: false,
            ..ClipboardPolicy::default()
        };
        assert!(matches!(
            check_otp_policy(&disabled, true),
            Err(KeynearError::Validation {
                field: "otpPassthroughEnabled",
                ..
            })
        ));
        assert!(check_otp_policy(&disabled, false).is_ok());
    }
}
