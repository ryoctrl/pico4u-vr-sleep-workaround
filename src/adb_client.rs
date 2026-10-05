use std::io::ErrorKind;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Manager};
use tauri_plugin_shell::ShellExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const ADB_PORT: u16 = 5037;
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    pub serial: String,
    pub state: String,
}

impl DeviceEntry {
    /// Network devices are listed as `host:port`; everything else is a USB transport.
    pub fn is_usb(&self) -> bool {
        !self.serial.contains(':')
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    Open,
    Refused,
    Unreachable,
}

/// Runs a `host:*` service and returns its length-prefixed payload.
pub async fn host_command(app: &AppHandle, command: &str) -> Result<String, String> {
    timeout(TIMEOUT, async {
        let mut stream = connect_adb(app).await?;
        host_request(&mut stream, command).await
    })
    .await
    .map_err(|_| format!("ADB host command '{}' timed out", command))?
}

/// Switches to the transport of `serial` and runs a device service, returning everything it writes.
pub async fn device_command(
    app: &AppHandle,
    serial: &str,
    command: &str,
) -> Result<String, String> {
    timeout(TIMEOUT, async {
        let mut stream = connect_adb(app).await?;
        device_request(&mut stream, serial, command).await
    })
    .await
    .map_err(|_| format!("ADB command '{}' on {} timed out", command, serial))?
}

pub async fn list_devices(app: &AppHandle) -> Result<Vec<DeviceEntry>, String> {
    Ok(parse_devices(&host_command(app, "host:devices").await?))
}

/// Opens a plain TCP connection to `addr` and closes it immediately.
pub async fn tcp_probe(addr: &str, wait: Duration) -> ProbeResult {
    match timeout(wait, TcpStream::connect(addr)).await {
        Ok(Ok(_)) => ProbeResult::Open,
        Ok(Err(e)) => classify_connect_error(e.kind()),
        Err(_) => ProbeResult::Unreachable,
    }
}

/// A refused connection means the host answered but nothing listens on the port.
/// Everything else (timeouts, no route, ...) is treated as the host not being reachable.
pub fn classify_connect_error(kind: ErrorKind) -> ProbeResult {
    match kind {
        ErrorKind::ConnectionRefused => ProbeResult::Refused,
        _ => ProbeResult::Unreachable,
    }
}

pub fn parse_devices(output: &str) -> Vec<DeviceEntry> {
    output
        .lines()
        .filter_map(|line| {
            let (serial, state) = line.trim().split_once(char::is_whitespace)?;
            Some(DeviceEntry {
                serial: serial.to_string(),
                state: state.trim().to_string(),
            })
        })
        .collect()
}

pub fn is_connect_success(output: &str) -> bool {
    let lower = output.to_ascii_lowercase();
    (lower.contains("connected to") || lower.contains("already connected"))
        && !lower.contains("failed")
        && !lower.contains("cannot")
}

/// Extracts the first IPv4 address from `ip -f inet addr show wlan0` output.
pub fn parse_wlan_ip(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("inet ")?;
        let addr = rest.split_whitespace().next()?.split('/').next()?;
        valid_ipv4(addr)
    })
}

/// Extracts the source address from `ip route get <addr>` output (`... src 192.168.1.10 uid 0`).
pub fn parse_route_src(output: &str) -> Option<String> {
    let mut words = output.split_whitespace();
    while let Some(word) = words.next() {
        if word == "src" {
            return valid_ipv4(words.next()?);
        }
    }
    None
}

fn valid_ipv4(s: &str) -> Option<String> {
    s.trim()
        .parse::<std::net::Ipv4Addr>()
        .ok()
        .map(|ip| ip.to_string())
}

async fn host_request<S>(stream: &mut S, command: &str) -> Result<String, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    send_request(stream, command).await?;
    read_status(stream, command).await?;

    let mut len_buf = [0u8; 4];
    if stream.read_exact(&mut len_buf).await.is_err() {
        return Ok(String::new());
    }
    read_hex_payload(stream, &len_buf).await
}

async fn device_request<S>(stream: &mut S, serial: &str, command: &str) -> Result<String, String>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let transport = format!("host:transport:{}", serial);
    send_request(stream, &transport).await?;
    read_status(stream, &transport).await?;

    send_request(stream, command).await?;
    read_status(stream, command).await?;

    let mut output = Vec::new();
    stream
        .read_to_end(&mut output)
        .await
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&output).to_string())
}

async fn send_request<S: AsyncWrite + Unpin>(stream: &mut S, command: &str) -> Result<(), String> {
    let req = format!("{:04x}{}", command.len(), command);
    stream
        .write_all(req.as_bytes())
        .await
        .map_err(|e| e.to_string())
}

async fn read_status<S: AsyncRead + Unpin>(stream: &mut S, command: &str) -> Result<(), String> {
    let mut status = [0u8; 4];
    stream
        .read_exact(&mut status)
        .await
        .map_err(|e| e.to_string())?;
    if &status == b"OKAY" {
        return Ok(());
    }

    // FAIL is followed by a length-prefixed reason.
    let mut len_buf = [0u8; 4];
    let reason = match stream.read_exact(&mut len_buf).await {
        Ok(_) => read_hex_payload(stream, &len_buf).await.unwrap_or_default(),
        Err(_) => String::new(),
    };
    Err(if reason.is_empty() {
        format!("ADB rejected '{}'", command)
    } else {
        format!("ADB rejected '{}': {}", command, reason)
    })
}

async fn read_hex_payload<S: AsyncRead + Unpin>(
    stream: &mut S,
    len_buf: &[u8; 4],
) -> Result<String, String> {
    let len_str = std::str::from_utf8(len_buf).map_err(|e| e.to_string())?;
    let len = usize::from_str_radix(len_str, 16).map_err(|e| e.to_string())?;
    let mut data = vec![0u8; len];
    stream
        .read_exact(&mut data)
        .await
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&data).to_string())
}

async fn connect_adb(app: &AppHandle) -> Result<TcpStream, String> {
    let addr = format!("127.0.0.1:{}", ADB_PORT);
    if let Ok(Ok(stream)) = timeout(TIMEOUT, TcpStream::connect(&addr)).await {
        return Ok(stream);
    }

    if let Ok(command) = app.shell().sidecar("adb") {
        let _ = command.args(["start-server"]).output().await;
        if let Ok(Ok(stream)) = timeout(TIMEOUT, TcpStream::connect(&addr)).await {
            // Mark that we started the server so we can safely kill it on exit
            let state = app.state::<crate::state::AppState>();
            state.adb_started_by_us.store(true, Ordering::SeqCst);
            return Ok(stream);
        }
    }
    Err("Failed to connect to ADB server and failed to start it.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::duplex;

    /// Plays the ADB server side: reads one request per entry and answers with the given bytes.
    async fn fake_server(
        mut server: tokio::io::DuplexStream,
        script: Vec<(&'static str, Vec<u8>)>,
    ) {
        for (expected, reply) in script {
            let mut len = [0u8; 4];
            server.read_exact(&mut len).await.unwrap();
            let len = usize::from_str_radix(std::str::from_utf8(&len).unwrap(), 16).unwrap();
            let mut body = vec![0u8; len];
            server.read_exact(&mut body).await.unwrap();
            assert_eq!(String::from_utf8(body).unwrap(), expected);
            server.write_all(&reply).await.unwrap();
        }
    }

    fn okay_with_payload(payload: &str) -> Vec<u8> {
        format!("OKAY{:04x}{}", payload.len(), payload).into_bytes()
    }

    #[tokio::test]
    async fn host_request_reads_payload() {
        let (mut client, server) = duplex(1024);
        let devices = "PA94XXXX\tdevice\n";
        let task = tokio::spawn(fake_server(
            server,
            vec![("host:devices", okay_with_payload(devices))],
        ));
        assert_eq!(
            host_request(&mut client, "host:devices").await.unwrap(),
            devices
        );
        task.await.unwrap();
    }

    #[tokio::test]
    async fn host_request_reports_fail_reason() {
        let (mut client, server) = duplex(1024);
        let reason = "device offline";
        let reply = format!("FAIL{:04x}{}", reason.len(), reason).into_bytes();
        let task = tokio::spawn(fake_server(
            server,
            vec![("host:connect:1.2.3.4:5555", reply)],
        ));
        let err = host_request(&mut client, "host:connect:1.2.3.4:5555")
            .await
            .unwrap_err();
        assert!(err.contains("device offline"), "{}", err);
        task.await.unwrap();
    }

    #[tokio::test]
    async fn device_request_switches_transport_then_reads_to_end() {
        let (mut client, server) = duplex(1024);
        let task = tokio::spawn(async move {
            fake_server(
                server,
                vec![
                    ("host:transport:PA94XXXX", b"OKAY".to_vec()),
                    ("shell:getprop ro.product.model", b"OKAYA9210\r\n".to_vec()),
                ],
            )
            .await;
            // Dropping the server end closes the stream so read_to_end finishes.
        });
        let out = device_request(&mut client, "PA94XXXX", "shell:getprop ro.product.model")
            .await
            .unwrap();
        assert_eq!(out.trim(), "A9210");
        task.await.unwrap();
    }

    #[tokio::test]
    async fn device_request_reports_transport_failure() {
        let (mut client, server) = duplex(1024);
        let reason = "device 'X' not found";
        let reply = format!("FAIL{:04x}{}", reason.len(), reason).into_bytes();
        let task = tokio::spawn(fake_server(server, vec![("host:transport:X", reply)]));
        let err = device_request(&mut client, "X", "shell:true")
            .await
            .unwrap_err();
        assert!(err.contains("not found"), "{}", err);
        task.await.unwrap();
    }

    #[test]
    fn parses_device_list() {
        let out = "PA94XXXX\tdevice\r\n192.168.1.10:5555\toffline\nemulator-5554\tno permissions (udev)\n\n";
        let devices = parse_devices(out);
        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].serial, "PA94XXXX");
        assert_eq!(devices[0].state, "device");
        assert!(devices[0].is_usb());
        assert_eq!(devices[1].state, "offline");
        assert!(!devices[1].is_usb());
        assert_eq!(devices[2].state, "no permissions (udev)");
    }

    #[test]
    fn parses_empty_device_list() {
        assert!(parse_devices("").is_empty());
    }

    #[test]
    fn judges_connect_output() {
        assert!(is_connect_success("connected to 192.168.1.10:5555"));
        assert!(is_connect_success("already connected to 192.168.1.10:5555"));
        assert!(!is_connect_success(
            "failed to connect to '192.168.1.10:5555': Connection refused"
        ));
        assert!(!is_connect_success("cannot connect to 192.168.1.10:5555"));
        assert!(!is_connect_success(""));
    }

    #[test]
    fn parses_wlan_ip() {
        let out = "29: wlan0: <BROADCAST,MULTICAST,UP> mtu 1500\r\n    inet 192.168.1.10/24 brd 192.168.1.255 scope global wlan0\r\n";
        assert_eq!(parse_wlan_ip(out).as_deref(), Some("192.168.1.10"));
        assert_eq!(parse_wlan_ip("Device \"wlan0\" does not exist."), None);
    }

    #[test]
    fn parses_route_src() {
        let out =
            "1.1.1.1 via 192.168.1.1 dev wlan0 table 1021 src 192.168.1.10 uid 0 \r\n    cache\r\n";
        assert_eq!(parse_route_src(out).as_deref(), Some("192.168.1.10"));
        assert_eq!(
            parse_route_src("RTNETLINK answers: Network is unreachable"),
            None
        );
    }

    #[test]
    fn classifies_connect_errors() {
        assert_eq!(
            classify_connect_error(ErrorKind::ConnectionRefused),
            ProbeResult::Refused
        );
        for kind in [
            ErrorKind::TimedOut,
            ErrorKind::HostUnreachable,
            ErrorKind::NetworkUnreachable,
            ErrorKind::AddrNotAvailable,
            ErrorKind::Other,
        ] {
            assert_eq!(classify_connect_error(kind), ProbeResult::Unreachable);
        }
    }

    #[tokio::test]
    async fn probe_detects_open_and_refused_ports() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        assert_eq!(
            tcp_probe(&addr, Duration::from_secs(2)).await,
            ProbeResult::Open
        );
        drop(listener);
        assert_eq!(
            tcp_probe(&addr, Duration::from_secs(2)).await,
            ProbeResult::Refused
        );
    }
}
