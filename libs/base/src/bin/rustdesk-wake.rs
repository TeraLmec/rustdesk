use base::wake::{self, HelperConfig};
use hbb_common::{anyhow::bail, ResultType};
use std::{
    net::TcpListener,
    path::Path,
    time::{Duration, Instant},
};

fn main() {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}

fn run() -> ResultType<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["keygen"] => println!("{}", wake::generate_key()?),
        ["check", file] => {
            HelperConfig::load(Path::new(file))?;
            println!("Wake helper configuration is valid");
        }
        ["wake", mac, broadcast] => wake::send_magic_packet(mac, wake::broadcast_address(broadcast)?)?,
        ["request", helper, device, key_file] => {
            let key = std::fs::read_to_string(key_file)?;
            wake::request_wake(helper, device, key.trim())?;
        }
        ["serve", file] => {
            let path = Path::new(file);
            let initial = HelperConfig::load(path)?;
            let listener = TcpListener::bind(initial.listen)?;
            eprintln!("Wake helper listening on {}", listener.local_addr()?);
            for stream in listener.incoming() {
                let started = Instant::now();
                // Reload authorization on every request so revoked keys stop working immediately.
                let result = HelperConfig::load(path).and_then(|config| {
                    wake::serve_connection(stream?, &config)
                });
                if let Err(err) = result {
                    eprintln!("Wake request failed: {err}");
                }
                // One bounded request at a time; no per-source caches or unbounded worker queue.
                if let Some(delay) = Duration::from_secs(1).checked_sub(started.elapsed()) {
                    std::thread::sleep(delay);
                }
            }
        }
        _ => bail!("Usage: rustdesk-wake keygen | check CONFIG | serve CONFIG | wake MAC BROADCAST:PORT | request HELPER:PORT DEVICE KEYFILE"),
    }
    Ok(())
}
