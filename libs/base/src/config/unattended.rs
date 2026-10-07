use hbb_common::{bail, config::Config, ResultType};

use super::keys::OPTION_ALLOW_UNATTENDED_ACCESS;

pub const CAPTURE_REQUIRED: &str = "Unattended screen capture is unavailable. Use X11 or install a RustDesk build with working DRM capture on the remote device. No screen sharing prompt was opened.";
pub const PASSWORD_REQUIRED: &str = "Unattended access requires password acceptance and an enabled permanent password on the remote device.";
pub const WINDOW_REQUIRED: &str = "Disable the requirement to keep the RustDesk window open on the remote device before using unattended access.";
pub const SERVICE_REQUIRED: &str =
    "Install and start the RustDesk service on the remote device before using unattended access.";
pub const INPUT_REQUIRED: &str = "Unattended input control is unavailable on the remote device. Check the RustDesk service and input permissions.";
pub const STARTUP_REQUIRED: &str =
    "Enable automatic startup for the RustDesk system service before using unattended access.";
pub const STARTUP_UNKNOWN: &str =
    "Could not verify automatic startup for the RustDesk system service.";

pub fn enabled() -> bool {
    cfg!(any(target_os = "windows", target_os = "linux"))
        && Config::get_option(OPTION_ALLOW_UNATTENDED_ACCESS) == "Y"
}

pub fn check_settings(
    approve_mode: &str,
    permanent_enabled: bool,
    permanent_set: bool,
    requires_window: bool,
) -> Result<(), &'static str> {
    if approve_mode != "password" || !permanent_enabled || !permanent_set {
        return Err(PASSWORD_REQUIRED);
    }
    if requires_window {
        return Err(WINDOW_REQUIRED);
    }
    Ok(())
}

pub fn check_interactive_capture() -> ResultType<()> {
    // Restore tokens can be revoked or rejected, so even a saved portal session may prompt.
    if enabled() {
        bail!(CAPTURE_REQUIRED);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unattended_access_requires_reusable_password_authentication() {
        assert_eq!(check_settings("password", true, true, false), Ok(()));
        for (mode, enabled, set) in [
            ("click", true, true),
            ("both", true, true),
            ("password", false, true),
            ("password", true, false),
        ] {
            assert_eq!(
                check_settings(mode, enabled, set, false),
                Err(PASSWORD_REQUIRED)
            );
        }
    }

    #[test]
    fn unattended_access_cannot_require_an_open_target_window() {
        assert_eq!(
            check_settings("password", true, true, true),
            Err(WINDOW_REQUIRED)
        );
    }
}
