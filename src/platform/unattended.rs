use hbb_common::ResultType;

#[cfg(target_os = "linux")]
pub fn automatic_start_enabled() -> ResultType<bool> {
    use hbb_common::anyhow::bail;
    use std::process::Command;

    let unit = format!("{}.service", crate::get_app_name().to_lowercase());
    let output = Command::new("systemctl")
        .args(["show", "--property=UnitFileState", "--value", &unit])
        .output()?;
    if !output.status.success() {
        bail!("Could not query automatic service startup");
    }
    Ok(String::from_utf8(output.stdout)?.trim() == "enabled")
}

#[cfg(target_os = "windows")]
pub fn automatic_start_enabled() -> ResultType<bool> {
    use winreg::{enums::HKEY_LOCAL_MACHINE, RegKey};

    let service = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(format!(
        "SYSTEM\\CurrentControlSet\\Services\\{}",
        crate::get_app_name()
    ))?;
    let start: u32 = service.get_value("Start")?;
    Ok(start == 2)
}
