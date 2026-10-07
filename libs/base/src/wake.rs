//! Wake-on-LAN and the optional, independently hosted wake helper.
use hbb_common::{
    anyhow::{anyhow, bail, Context},
    sodiumoxide::{self, crypto::secretbox, randombytes},
    ResultType,
};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpStream, ToSocketAddrs, UdpSocket},
    path::Path,
    time::{Duration, Instant},
};

const MAX_FRAME: usize = 4096;
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default)]
    pub mac: String,
    #[serde(default)]
    pub broadcast: String,
    #[serde(default)]
    pub helper: String,
    #[serde(default)]
    pub device: String,
    #[serde(default)]
    pub key: String,
}

impl Profile {
    pub fn validate(&self) -> ResultType<()> {
        if self.helper.is_empty() {
            magic_packet(&self.mac)?;
            broadcast_address(&self.broadcast)?;
        } else {
            if self.helper.len() > 253 || self.device.is_empty() || self.device.len() > 128 {
                bail!("Invalid wake helper address or device name");
            }
            let _key = parse_key(&self.key)?;
        }
        Ok(())
    }

    pub fn wake(&self) -> ResultType<()> {
        self.validate()?;
        if self.helper.is_empty() {
            send_magic_packet(&self.mac, broadcast_address(&self.broadcast)?)
        } else {
            request_wake(&self.helper, &self.device, &self.key)
        }
    }
}

pub fn magic_packet(mac: &str) -> ResultType<[u8; 102]> {
    let compact = mac.replace([':', '-'], "");
    let bytes = decode_hex::<6>(&compact).context("Enter a valid Ethernet MAC address")?;
    if bytes == [0; 6] || bytes[0] & 1 != 0 {
        bail!("Enter a unicast Ethernet MAC address");
    }
    let mut packet = [0xff; 102];
    for chunk in packet[6..].chunks_exact_mut(6) {
        chunk.copy_from_slice(&bytes);
    }
    Ok(packet)
}

pub fn broadcast_address(value: &str) -> ResultType<SocketAddrV4> {
    let value = if value.is_empty() {
        "255.255.255.255:9"
    } else {
        value
    };
    let address: SocketAddrV4 = value
        .parse()
        .context("Enter an IPv4 broadcast address and port")?;
    if address.port() == 0 || address.ip().is_unspecified() || address.ip().is_multicast() {
        bail!("Invalid wake destination");
    }
    Ok(address)
}

pub fn send_magic_packet(mac: &str, destination: SocketAddrV4) -> ResultType<()> {
    let packet = magic_packet(mac)?;
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.set_broadcast(true)?;
    socket.set_write_timeout(Some(TIMEOUT))?;
    for _ in 0..3 {
        if socket.send_to(&packet, destination)? != packet.len() {
            bail!("Incomplete wake packet");
        }
    }
    Ok(())
}

pub fn generate_key() -> ResultType<String> {
    sodiumoxide::init().map_err(|_| anyhow!("Could not initialize wake authentication"))?;
    Ok(encode_hex(&secretbox::gen_key().0))
}

fn parse_key(value: &str) -> ResultType<secretbox::Key> {
    Ok(secretbox::Key(decode_hex::<32>(value).context(
        "Wake key must contain 64 hexadecimal characters",
    )?))
}

fn decode_hex<const N: usize>(value: &str) -> ResultType<[u8; N]> {
    if value.len() != N * 2 || !value.bytes().all(|c| c.is_ascii_hexdigit()) {
        bail!("Invalid hexadecimal value");
    }
    let mut result = [0; N];
    for (i, byte) in result.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[i * 2..i * 2 + 2], 16)?;
    }
    Ok(result)
}

fn encode_hex(value: &[u8]) -> String {
    value.iter().map(|v| format!("{v:02x}")).collect()
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    device: String,
    nonce: [u8; 24],
    ciphertext: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Command {
    challenge: [u8; 32],
    device: String,
    operation: String,
}

#[derive(Serialize, Deserialize)]
struct Reply {
    challenge: [u8; 32],
    result: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelperConfig {
    pub listen: SocketAddr,
    pub devices: BTreeMap<String, Device>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub mac: String,
    pub broadcast: SocketAddrV4,
    // Two keys allow rotation: add a replacement, update clients, remove the old key.
    pub keys: Vec<String>,
}

impl HelperConfig {
    pub fn load(path: &Path) -> ResultType<Self> {
        let file = std::fs::File::open(path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if file.metadata()?.permissions().mode() & 0o077 != 0 {
                bail!("Wake helper configuration must be readable only by its owner (chmod 600)");
            }
        }
        let mut contents = Vec::new();
        file.take(131_073).read_to_end(&mut contents)?;
        if contents.len() > 131_072 {
            bail!("Wake helper configuration is too large");
        }
        let config: Self = serde_json::from_slice(&contents)
            .map_err(|_| anyhow!("Invalid wake helper configuration JSON"))?;
        if config.devices.is_empty() || config.devices.len() > 128 {
            bail!("Configure between 1 and 128 wake devices");
        }
        for (name, device) in &config.devices {
            if name.is_empty() || name.len() > 128 || !(1..=2).contains(&device.keys.len()) {
                bail!("Invalid wake device name or key count");
            }
            magic_packet(&device.mac)?;
            broadcast_address(&device.broadcast.to_string())?;
            for key in &device.keys {
                let _key = parse_key(key)?;
            }
        }
        Ok(config)
    }
}

fn configure_stream(stream: &TcpStream) -> ResultType<()> {
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    Ok(())
}

fn write_frame(stream: &mut TcpStream, data: &[u8]) -> ResultType<()> {
    if data.len() > MAX_FRAME {
        bail!("Wake message is too large");
    }
    stream.write_all(&(data.len() as u32).to_be_bytes())?;
    stream.write_all(data)?;
    Ok(())
}

fn read_frame(stream: &mut TcpStream) -> ResultType<Vec<u8>> {
    fn read_until(
        stream: &mut TcpStream,
        mut output: &mut [u8],
        deadline: Instant,
    ) -> ResultType<()> {
        while !output.is_empty() {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .filter(|duration| !duration.is_zero())
                .ok_or_else(|| anyhow!("Wake helper request timed out"))?;
            stream.set_read_timeout(Some(remaining))?;
            let count = stream.read(output)?;
            if count == 0 {
                bail!("Wake helper connection closed");
            }
            output = &mut output[count..];
        }
        Ok(())
    }
    let deadline = Instant::now() + TIMEOUT;
    let mut header = [0; 4];
    read_until(stream, &mut header, deadline)?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME {
        bail!("Wake message is too large");
    }
    let mut data = vec![0; length];
    read_until(stream, &mut data, deadline)?;
    Ok(data)
}

fn authorize<'a>(
    config: &'a HelperConfig,
    challenge: [u8; 32],
    request: &Request,
) -> ResultType<(&'a Device, secretbox::Key)> {
    let device = config
        .devices
        .get(&request.device)
        .ok_or_else(|| anyhow!("Wake authentication failed"))?;
    for value in &device.keys {
        let key = parse_key(value)?;
        if let Ok(bytes) =
            secretbox::open(&request.ciphertext, &secretbox::Nonce(request.nonce), &key)
        {
            let command: Command = serde_json::from_slice(&bytes)?;
            if command.challenge == challenge
                && command.device == request.device
                && command.operation == "rustdesk-wake-v1"
            {
                return Ok((device, key));
            }
        }
    }
    bail!("Wake authentication failed")
}

/// Exactly one fresh challenge and one authenticated operation per connection.
pub fn serve_connection(mut stream: TcpStream, config: &HelperConfig) -> ResultType<()> {
    sodiumoxide::init().map_err(|_| anyhow!("Could not initialize wake authentication"))?;
    configure_stream(&stream)?;
    let mut challenge = [0; 32];
    randombytes::randombytes_into(&mut challenge);
    write_frame(&mut stream, &challenge)?;
    let request: Request = serde_json::from_slice(&read_frame(&mut stream)?)?;
    let (device, key) = authorize(config, challenge, &request)?;
    let result = match send_magic_packet(&device.mac, device.broadcast) {
        Ok(()) => "sent".to_owned(),
        Err(_) => "Wake helper could not send the LAN packet".to_owned(),
    };
    let nonce = secretbox::gen_nonce();
    let ciphertext = secretbox::seal(
        &serde_json::to_vec(&Reply { challenge, result })?,
        &nonce,
        &key,
    );
    let mut response = nonce.0.to_vec();
    response.extend(ciphertext);
    write_frame(&mut stream, &response)
}

pub fn request_wake(helper: &str, device: &str, key: &str) -> ResultType<()> {
    sodiumoxide::init().map_err(|_| anyhow!("Could not initialize wake authentication"))?;
    let key = parse_key(key)?;
    let addresses: Vec<_> = helper.to_socket_addrs()?.take(4).collect();
    let mut connection = None;
    for address in addresses {
        if let Ok(stream) = TcpStream::connect_timeout(&address, TIMEOUT) {
            connection = Some(stream);
            break;
        }
    }
    let mut stream = connection.ok_or_else(|| anyhow!("Wake helper is unreachable"))?;
    configure_stream(&stream)?;
    let bytes = read_frame(&mut stream)?;
    if bytes.len() != 32 {
        bail!("Invalid wake helper challenge");
    }
    let mut challenge = [0; 32];
    challenge.copy_from_slice(&bytes);
    let nonce = secretbox::gen_nonce();
    let command = Command {
        challenge,
        device: device.to_owned(),
        operation: "rustdesk-wake-v1".to_owned(),
    };
    let request = Request {
        device: device.to_owned(),
        nonce: nonce.0,
        ciphertext: secretbox::seal(&serde_json::to_vec(&command)?, &nonce, &key),
    };
    write_frame(&mut stream, &serde_json::to_vec(&request)?)?;
    let response = read_frame(&mut stream)?;
    if response.len() < secretbox::NONCEBYTES {
        bail!("Invalid wake helper reply");
    }
    let nonce = secretbox::Nonce::from_slice(&response[..secretbox::NONCEBYTES])
        .ok_or_else(|| anyhow!("Invalid wake helper reply"))?;
    let bytes = secretbox::open(&response[secretbox::NONCEBYTES..], &nonce, &key)
        .map_err(|_| anyhow!("Wake helper authentication failed"))?;
    let reply: Reply = serde_json::from_slice(&bytes)?;
    if reply.challenge != challenge {
        bail!("Invalid wake helper reply");
    }
    if reply.result != "sent" {
        bail!(reply.result);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sends_the_requested_magic_packet() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket.set_read_timeout(Some(TIMEOUT)).unwrap();
        let address = match socket.local_addr().unwrap() {
            SocketAddr::V4(a) => a,
            _ => unreachable!(),
        };
        send_magic_packet("02:11:22:33:44:55", address).unwrap();
        let mut packet = [0; 102];
        assert_eq!(socket.recv(&mut packet).unwrap(), 102);
        assert_eq!(&packet[..6], &[255; 6]);
        for part in packet[6..].chunks_exact(6) {
            assert_eq!(part, &[2, 17, 34, 51, 68, 85]);
        }
        assert!(magic_packet("ff:ff:ff:ff:ff:ff").is_err());
        assert!(magic_packet("invalid").is_err());
    }

    #[test]
    fn helper_rejects_replay_wrong_keys_and_other_devices() {
        let key_text = generate_key().unwrap();
        let key = parse_key(&key_text).unwrap();
        let mut config = HelperConfig {
            listen: "127.0.0.1:0".parse().unwrap(),
            devices: BTreeMap::from([(
                "desktop".to_owned(),
                Device {
                    mac: "02:11:22:33:44:55".to_owned(),
                    broadcast: "127.0.0.1:9".parse().unwrap(),
                    keys: vec![key_text],
                },
            )]),
        };
        let command = Command {
            challenge: [1; 32],
            device: "desktop".to_owned(),
            operation: "rustdesk-wake-v1".to_owned(),
        };
        let nonce = secretbox::gen_nonce();
        let mut request = Request {
            device: "desktop".to_owned(),
            nonce: nonce.0,
            ciphertext: secretbox::seal(&serde_json::to_vec(&command).unwrap(), &nonce, &key),
        };
        assert!(authorize(&config, [1; 32], &request).is_ok());
        assert!(authorize(&config, [2; 32], &request).is_err());
        request.device = "other".to_owned();
        assert!(authorize(&config, [1; 32], &request).is_err());
        request.device = "desktop".to_owned();
        request.ciphertext[0] ^= 1;
        assert!(authorize(&config, [1; 32], &request).is_err());
        let replacement = generate_key().unwrap();
        request.ciphertext = secretbox::seal(
            &serde_json::to_vec(&command).unwrap(),
            &nonce,
            &parse_key(&replacement).unwrap(),
        );
        assert!(authorize(&config, [1; 32], &request).is_err());
        config
            .devices
            .get_mut("desktop")
            .unwrap()
            .keys
            .push(replacement.clone());
        assert!(authorize(&config, [1; 32], &request).is_ok());
        config.devices.get_mut("desktop").unwrap().keys = vec![replacement];
        request.ciphertext = secretbox::seal(&serde_json::to_vec(&command).unwrap(), &nonce, &key);
        assert!(authorize(&config, [1; 32], &request).is_err());
    }

    #[test]
    fn authenticated_helper_delivers_wake_and_acknowledges() {
        use std::net::TcpListener;
        let udp = UdpSocket::bind("127.0.0.1:0").unwrap();
        udp.set_read_timeout(Some(TIMEOUT)).unwrap();
        let key = generate_key().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let config: HelperConfig = serde_json::from_value(serde_json::json!({"listen": address, "devices": {"desktop": {"mac": "02:11:22:33:44:55", "broadcast": udp.local_addr().unwrap(), "keys": [key]}}})).unwrap();
        let task =
            std::thread::spawn(move || serve_connection(listener.accept().unwrap().0, &config));
        request_wake(&address.to_string(), "desktop", &key).unwrap();
        task.join().unwrap().unwrap();
        let mut packet = [0; 102];
        assert_eq!(udp.recv(&mut packet).unwrap(), 102);
    }
}
