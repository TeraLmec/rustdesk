use base::config::{keys, unattended};
use hbb_common::{config::Config, password_security};

pub(super) fn check_settings() -> Result<(), &'static str> {
    if !unattended::enabled() {
        return Ok(());
    }
    unattended::check_settings(
        &Config::get_option(keys::OPTION_APPROVE_MODE),
        password_security::permanent_enabled(),
        Config::has_permanent_password(),
        Config::get_bool_option(keys::OPTION_ALLOW_ONLY_CONN_WINDOW_OPEN),
    )
}

pub(super) async fn check_runtime() -> Result<(), &'static str> {
    if !unattended::enabled() {
        return Ok(());
    }
    if crate::ipc::connect_service(1_000).await.is_err() {
        return Err(unattended::SERVICE_REQUIRED);
    }
    match hbb_common::tokio::task::spawn_blocking(
        crate::platform::unattended::automatic_start_enabled,
    )
    .await
    {
        Ok(Ok(true)) => {}
        Ok(Ok(false)) => return Err(unattended::STARTUP_REQUIRED),
        _ => return Err(unattended::STARTUP_UNKNOWN),
    }
    #[cfg(target_os = "windows")]
    if !crate::platform::is_installed() || !crate::platform::is_self_service_running() {
        return Err(unattended::SERVICE_REQUIRED);
    }
    #[cfg(target_os = "linux")]
    if !crate::platform::linux::is_x11() {
        #[cfg(feature = "drm")]
        {
            super::drm_capturer::settle_unknown_availability().await;
            if !super::drm_capturer::is_available_cached() {
                return Err(unattended::CAPTURE_REQUIRED);
            }
        }
        #[cfg(not(feature = "drm"))]
        return Err(unattended::CAPTURE_REQUIRED);

        #[cfg(feature = "drm")]
        if !matches!(
            hbb_common::tokio::task::spawn_blocking(super::input_service::unattended_input_ready)
                .await,
            Ok(true)
        ) {
            return Err(unattended::INPUT_REQUIRED);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub(super) fn capture_error(
    service: &super::GenericService,
    err: hbb_common::anyhow::Error,
) -> hbb_common::anyhow::Error {
    use base::message_proto::{Message, Misc};
    use std::sync::Arc;

    if unattended::enabled() && err.to_string() == unattended::CAPTURE_REQUIRED {
        let mut misc = Misc::new();
        misc.set_close_reason(unattended::CAPTURE_REQUIRED.to_owned());
        let mut msg = Message::new();
        msg.set_misc(misc);
        let msg = Arc::new(msg);
        service.send_shared(msg.clone());
        // Capture can fail before the first frame promotes new subscribers.
        if let Err(notify_err) = service.snapshot(|subscribers| {
            subscribers.send_shared(msg.clone());
            Ok(())
        }) {
            hbb_common::throttled_log!(
                std::time::Duration::from_secs(60),
                warn,
                "Failed to report unattended capture failure: {notify_err}"
            );
        }
    }
    err
}
