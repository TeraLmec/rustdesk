use base::{config::keys, wake::Profile};
use hbb_common::{
    anyhow::{anyhow, bail},
    config::{LanPeers, PeerConfig},
    password_security::{decrypt_str_or_original, encrypt_str_or_original},
    ResultType,
};
use serde_json::{json, Value};

pub fn handle_request(request: &str) -> String {
    match execute(request) {
        Ok(value) => json!({"ok": true, "value": value}).to_string(),
        Err(error) => json!({"ok": false, "error": error.to_string()}).to_string(),
    }
}

fn load_profile(id: &str) -> ResultType<Profile> {
    let peer = PeerConfig::load(id);
    if let Some(value) = peer.options.get(keys::OPTION_WAKE_PROFILE) {
        return Ok(serde_json::from_str(value)?);
    }
    let mac = LanPeers::load()
        .peers
        .iter()
        .find(|peer| peer.id == id)
        .and_then(|peer| {
            peer.ip_mac
                .values()
                .find(|mac| base::wake::magic_packet(mac).is_ok())
                .cloned()
        })
        .unwrap_or_default();
    Ok(Profile {
        mac,
        broadcast: "255.255.255.255:9".to_owned(),
        ..Default::default()
    })
}

fn execute(request: &str) -> ResultType<Value> {
    if request.len() > 8192 {
        bail!("Wake configuration is too large");
    }
    let request: Value = serde_json::from_str(request)?;
    let id = request["id"]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 256)
        .ok_or_else(|| anyhow!("Missing peer ID"))?;
    match request["action"].as_str() {
        Some("load") => {
            let mut profile = load_profile(id)?;
            let has_key = !profile.key.is_empty();
            profile.key.clear();
            Ok(json!({"profile": profile, "has_key": has_key}))
        }
        Some("save") => {
            let mut profile: Profile = serde_json::from_value(request["profile"].clone())?;
            if profile.helper.is_empty() {
                profile.key.clear();
                profile.validate()?;
            } else {
                let old = load_profile(id)?;
                if profile.key.is_empty() {
                    if profile.helper != old.helper || profile.device != old.device {
                        bail!("Enter a wake key for the new helper or device");
                    }
                    profile.key = decrypt_str_or_original(&old.key, "00")
                        .0
                        .strip_prefix("wake:")
                        .ok_or_else(|| anyhow!("Enter the wake key again"))?
                        .to_owned();
                }
                profile.validate()?;
                let plaintext = format!("wake:{}", profile.key);
                let encrypted = encrypt_str_or_original(&plaintext, "00", 128);
                if encrypted.is_empty() || encrypted == plaintext {
                    bail!("Could not protect the wake key on this device");
                }
                profile.key = encrypted;
            }
            let mut peer = PeerConfig::load(id);
            peer.options.insert(
                keys::OPTION_WAKE_PROFILE.to_owned(),
                serde_json::to_string(&profile)?,
            );
            peer.options.insert(
                keys::OPTION_ALLOW_UNATTENDED_RECONNECT.to_owned(),
                "Y".to_owned(),
            );
            peer.store(id);
            let saved = PeerConfig::load(id);
            if saved.options.get(keys::OPTION_WAKE_PROFILE)
                != peer.options.get(keys::OPTION_WAKE_PROFILE)
                || saved
                    .options
                    .get(keys::OPTION_ALLOW_UNATTENDED_RECONNECT)
                    .map(String::as_str)
                    != Some("Y")
            {
                bail!("Could not save wake settings");
            }
            Ok(Value::Null)
        }
        Some("wake") => {
            let mut profile = load_profile(id)?;
            if !profile.helper.is_empty() {
                let (key, decrypted, _) = decrypt_str_or_original(&profile.key, "00");
                if !decrypted {
                    bail!("The saved wake key cannot be read. Enter it again in wake settings.");
                }
                profile.key = key
                    .strip_prefix("wake:")
                    .ok_or_else(|| anyhow!("Enter the wake key again"))?
                    .to_owned();
            }
            profile.wake()?;
            Ok(Value::Null)
        }
        _ => bail!("Unknown wake action"),
    }
}
