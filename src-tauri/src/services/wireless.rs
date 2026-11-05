use std::time::{Duration, Instant};

use crate::error::{AppError, Result};
use crate::models::{Device, MdnsDevice, QrPayload, ReconnectTarget};
use crate::services::adb;
use crate::services::mdns::{self, MdnsService, CONNECT_SERVICE, PAIRING_SERVICE};
use crate::util::validation::{
    parse_host_port, validate_pairing_password, validate_serial, validate_service_name,
};

/// The port `adb tcpip` restarts adbd on by default.
pub const DEFAULT_TCPIP_PORT: u16 = 5555;

const MDNS_TIMEOUT: Duration = Duration::from_secs(10);
const SHELL_TIMEOUT: Duration = Duration::from_secs(15);
const PAIR_TIMEOUT: Duration = Duration::from_secs(30);
/// How long to keep looking for the (separate) TLS connect port after pairing.
const CONNECT_WAIT: Duration = Duration::from_secs(30);
const POLL_INTERVAL: Duration = Duration::from_millis(1000);

/// Build a fresh QR payload (random service name + numeric password) from `Uuid::new_v4()`, not the clock.
pub fn generate() -> QrPayload {
    let id = uuid::Uuid::new_v4();
    let compact = id.simple().to_string();
    let name = format!("zesp-{}", &compact[..10]);
    let password = numeric_password(&id);
    QrPayload {
        payload: format!("WIFI:T:ADB;S:{name};P:{password};;"),
        password,
        created: crate::util::time::unix_now(),
    }
}

fn numeric_password(id: &uuid::Uuid) -> String {
    id.as_bytes()
        .iter()
        .take(10)
        .map(|byte| char::from(b'0' + (byte % 10)))
        .collect()
}

/// Extract and validate the service name + password from a QR payload.
pub fn parse_qr(payload: &str) -> Result<(String, String)> {
    let mut name = None;
    let mut password = None;
    for field in payload.split(';') {
        if let Some(value) = field.strip_prefix("S:") {
            name = Some(value.to_string());
        } else if let Some(value) = field.strip_prefix("P:") {
            password = Some(value.to_string());
        }
    }

    let name =
        name.ok_or_else(|| AppError::Invalid("QR payload is missing the service name".into()))?;
    let password =
        password.ok_or_else(|| AppError::Invalid("QR payload is missing the password".into()))?;

    validate_service_name(&name)?;
    validate_pairing_password(&password)?;
    Ok((name, password))
}

/// Parse `adb mdns services` output into validated service entries.
fn parse_mdns(text: &str) -> Vec<MdnsService> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let service_type = parts.next()?;
            let address = parts.next()?;
            if !service_type.starts_with("_adb") {
                return None;
            }
            let (host, port) = parse_host_port(address).ok()?;
            Some(MdnsService {
                name: name.to_string(),
                service_type: service_type.to_string(),
                host,
                port,
            })
        })
        .collect()
}

/// One snapshot of `adb mdns services` merged with our own browser's services.
async fn mdns_services(adb_path: &str) -> Result<Vec<MdnsService>> {
    let args = vec!["mdns".to_string(), "services".to_string()];
    let output = adb::run(adb_path, &args, MDNS_TIMEOUT).await?;
    Ok(merge(parse_mdns(&output.stdout), mdns::services()))
}

fn merge(mut services: Vec<MdnsService>, extra: Vec<MdnsService>) -> Vec<MdnsService> {
    for service in extra {
        if !services.contains(&service) {
            services.push(service);
        }
    }
    services
}

/// The discovered services, shaped for the UI (with a pre-joined `address`).
pub async fn mdns_devices(adb_path: &str) -> Result<Vec<MdnsDevice>> {
    let services = mdns_services(adb_path).await?;
    Ok(services
        .into_iter()
        .map(|service| MdnsDevice {
            address: format!("{}:{}", service.host, service.port),
            name: service.name,
            service_type: service.service_type,
            host: service.host,
            port: service.port,
        })
        .collect())
}

/// Poll mdns until a service matching `matches` appears or the deadline passes.
async fn wait_for<F>(adb_path: &str, deadline: Instant, mut matches: F) -> Result<MdnsService>
where
    F: FnMut(&MdnsService) -> bool,
{
    loop {
        if let Ok(services) = mdns_services(adb_path).await {
            if let Some(found) = services.into_iter().find(|service| matches(service)) {
                return Ok(found);
            }
        }
        if Instant::now() >= deadline {
            return Err(AppError::Other(
                "Timed out waiting for the phone. Make sure it is on the same Wi-Fi \
                 network and that you scanned the QR from Wireless debugging."
                    .into(),
            ));
        }
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Pair with the phone behind `payload`, then connect it; returns the connected `host:port`.
pub async fn pair_and_connect(adb_path: &str, payload: &str, timeout: Duration) -> Result<String> {
    let (name, password) = parse_qr(payload)?;

    adb::start_server(adb_path).await?;

    // The phone advertises the pairing service only after the QR is scanned.
    let pair_deadline = Instant::now() + timeout;
    let pairing = wait_for(adb_path, pair_deadline, |service| {
        service.service_type == PAIRING_SERVICE && service.name == name
    })
    .await?;
    let pairing_address = format!("{}:{}", pairing.host, pairing.port);

    pair(adb_path, &pairing_address, &password).await?;

    // The TLS connect port differs from the pairing port; discover and connect.
    let connect_deadline = Instant::now() + CONNECT_WAIT;
    if let Ok(service) = wait_for(adb_path, connect_deadline, |service| {
        service.service_type == CONNECT_SERVICE && service.host == pairing.host
    })
    .await
    {
        let target = format!("{}:{}", service.host, service.port);
        let connect_error = match connect(adb_path, &target).await {
            Ok(target) => return Ok(target),
            Err(error) => error,
        };
        // Recent adb may already have auto-connected it, listed under its address or mDNS name.
        if device_present(adb_path, &[&service.host, &service.name]).await {
            return Ok(target);
        }
        return Err(connect_error);
    }

    // Recent adb auto-connects to paired devices; accept that if it did.
    if device_present(adb_path, &[&pairing.host]).await {
        return Ok(pairing_address);
    }

    Err(AppError::Other(
        "Paired with the phone, but its debugging port was not discovered. Keep the \
         Wireless debugging screen open and try again."
            .into(),
    ))
}

async fn pair(adb_path: &str, address: &str, password: &str) -> Result<()> {
    let args = vec![
        "pair".to_string(),
        address.to_string(),
        password.to_string(),
    ];

    let output = adb::run(adb_path, &args, PAIR_TIMEOUT).await?;
    if paired(&output.stdout) || paired(&output.stderr) {
        return Ok(());
    }

    // A stale adb server can fault during the TLS handshake; restart once.
    let message = output.text();
    if is_protocol_fault(&message) {
        let _ = adb::kill_server(adb_path).await;
        adb::start_server(adb_path).await?;
        let retry = adb::run(adb_path, &args, PAIR_TIMEOUT).await?;
        if paired(&retry.stdout) || paired(&retry.stderr) {
            return Ok(());
        }
        return Err(AppError::Other(format!("Pairing failed: {}", retry.text())));
    }

    Err(AppError::Other(format!("Pairing failed: {message}")))
}

fn paired(output: &str) -> bool {
    output.contains("Successfully paired")
}

fn is_protocol_fault(message: &str) -> bool {
    let lowered = message.to_lowercase();
    lowered.contains("protocol fault") || lowered.contains("couldn't read status message")
}

/// Whether `adb devices` lists a device mentioning any of `needles`.
async fn device_present(adb_path: &str, needles: &[&str]) -> bool {
    match adb::list_devices_raw(adb_path).await {
        Ok(output) => output
            .stdout
            .lines()
            .any(|line| needles.iter().any(|needle| line.contains(needle))),
        Err(_) => false,
    }
}

/// Pair using an explicit `host:port` and the phone's pairing code; returns the normalized address.
pub async fn pair_code(adb_path: &str, address: &str, code: &str) -> Result<String> {
    let (host, port) = parse_host_port(address)?;
    validate_pairing_password(code)?;

    adb::start_server(adb_path).await?;
    let target = format!("{host}:{port}");
    pair(adb_path, &target, code).await?;
    Ok(target)
}

/// `adb connect <address>`. Returns the normalized `host:port` on success.
pub async fn connect(adb_path: &str, address: &str) -> Result<String> {
    let (host, port) = parse_host_port(address)?;
    let target = format!("{host}:{port}");

    let output = adb::connect(adb_path, &target).await?;
    let text = output.text();
    if connected(&text) {
        Ok(target)
    } else {
        Err(AppError::CommandFailed(format!(
            "Could not connect to {target}: {text}"
        )))
    }
}

pub async fn disconnect(adb_path: &str, target: &str) -> Result<()> {
    validate_serial(target)?;
    adb::disconnect(adb_path, target).await?;
    Ok(())
}

/// The result of reconnecting a saved phone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconnect {
    Connected(String),
    /// The phone answered but no longer trusts this computer; only pairing again helps.
    Unpaired,
    Unavailable(String),
}

/// The connect services currently announced, also asking adb's mDNS (spawns an adb process).
pub async fn connect_services(adb_path: &str) -> Vec<MdnsService> {
    only_connect(
        mdns_services(adb_path)
            .await
            .unwrap_or_else(|_| mdns::services()),
    )
}

/// The connect services our own browser has seen; free, no process spawned.
pub fn cached_connect_services() -> Vec<MdnsService> {
    only_connect(mdns::services())
}

fn only_connect(services: Vec<MdnsService>) -> Vec<MdnsService> {
    services
        .into_iter()
        .filter(|service| service.service_type == CONNECT_SERVICE)
        .collect()
}

/// Where a saved phone listens now: its announcement by stable name, else any on its last IP.
pub fn locate<'a>(
    services: &'a [MdnsService],
    target: &ReconnectTarget,
) -> Option<&'a MdnsService> {
    let by_name = target
        .name
        .as_deref()
        .and_then(|name| services.iter().find(|service| service.name == name));
    by_name.or_else(|| {
        let host = target.address.as_deref().and_then(host_of)?;
        services.iter().find(|service| service.host == host)
    })
}

/// Whether a saved phone already has a working wireless transport.
pub fn is_online(
    devices: &[Device],
    target: &ReconnectTarget,
    service: Option<&MdnsService>,
) -> bool {
    devices
        .iter()
        .filter(|device| device.connected)
        .any(|device| {
            device.serial == target.serial
                || (target.name.is_some() && device.mdns_name == target.name)
                || service.is_some_and(|service| {
                    device.address.as_deref().and_then(host_of) == Some(service.host.as_str())
                })
        })
}

/// A phone announced at `service` while a device still shows an older port of the same IP.
pub fn has_stale_port(devices: &[Device], service: &MdnsService) -> bool {
    let live = format!("{}:{}", service.host, service.port);
    devices.iter().any(|device| {
        device
            .address
            .as_deref()
            .is_some_and(|old| old != live && host_of(old) == Some(service.host.as_str()))
    })
}

/// Drop dead transports on old ports of `live`'s IP. Reads the raw `adb devices` list, as the
/// merged device list folds those transports into the live one. Returns whether any were dropped.
pub async fn drop_stale(adb_path: &str, live: &str) -> bool {
    let Some(host) = host_of(live) else {
        return false;
    };
    let Ok(output) = adb::list_devices_raw(adb_path).await else {
        return false;
    };
    let mut dropped = false;
    for serial in stale_serials(&output.stdout, host, live) {
        dropped |= adb::disconnect(adb_path, &serial)
            .await
            .is_ok_and(|output| output.success);
    }
    dropped
}

fn stale_serials(devices_output: &str, host: &str, live: &str) -> Vec<String> {
    devices_output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            Some((parts.next()?, parts.next()?))
        })
        .filter(|(serial, state)| {
            *state != "device" && *serial != live && host_of(serial) == Some(host)
        })
        .map(|(serial, _)| serial.to_string())
        .collect()
}

/// `adb connect` a saved phone at `address`, first dropping its dead transports on old ports.
pub async fn reconnect(adb_path: &str, address: &str) -> Reconnect {
    let Ok((host, port)) = parse_host_port(address) else {
        return Reconnect::Unavailable(format!("Invalid address {address}"));
    };
    let target = format!("{host}:{port}");
    drop_stale(adb_path, &target).await;

    match adb::connect(adb_path, &target).await {
        Ok(output) => classify_connect(&output.text(), target),
        Err(error) => Reconnect::Unavailable(error.to_string()),
    }
}

fn classify_connect(text: &str, target: String) -> Reconnect {
    if connected(text) {
        Reconnect::Connected(target)
    } else if text.to_lowercase().contains("authenticat") {
        Reconnect::Unpaired
    } else {
        Reconnect::Unavailable(format!("Could not connect to {target}: {text}"))
    }
}

fn host_of(address: &str) -> Option<&str> {
    address.rsplit_once(':').map(|(host, _)| host)
}

/// Restart adbd in TCP mode on a USB device, discover its Wi-Fi IP, and connect; returns the address.
pub async fn enable_tcpip(adb_path: &str, serial: &str, port: u16) -> Result<String> {
    validate_serial(serial)?;
    if port == 0 {
        return Err(AppError::Invalid("Invalid TCP port".into()));
    }

    let output = adb::tcpip(adb_path, serial, port).await?;
    let text = output.text();
    if !tcpip_enabled(&text) {
        return Err(AppError::CommandFailed(format!(
            "Could not enable wireless debugging on {serial}: {text}"
        )));
    }

    let ip = wifi_ip(adb_path, serial).await?;
    connect(adb_path, &format!("{ip}:{port}")).await
}

/// The device's Wi-Fi IPv4 address, via `ip` with a `getprop` fallback.
async fn wifi_ip(adb_path: &str, serial: &str) -> Result<String> {
    let output = adb::shell(
        adb_path,
        serial,
        "ip -f inet addr show wlan0",
        SHELL_TIMEOUT,
    )
    .await?;
    if let Some(ip) = parse_ip(&output.stdout) {
        return Ok(ip);
    }

    let fallback = adb::shell(
        adb_path,
        serial,
        "getprop dhcp.wlan0.ipaddress",
        SHELL_TIMEOUT,
    )
    .await?;
    parse_ip(&fallback.stdout).ok_or_else(|| {
        AppError::Other(
            "Could not determine the phone's Wi-Fi IP. Make sure Wi-Fi is on and try again.".into(),
        )
    })
}

/// True when `adb connect` reported success (fresh or already connected).
fn connected(text: &str) -> bool {
    let lowered = text.to_lowercase();
    lowered.contains("connected to") || lowered.contains("already connected")
}

/// True when `adb tcpip` restarted adbd in TCP mode.
fn tcpip_enabled(text: &str) -> bool {
    text.to_lowercase().contains("restarting in tcp mode")
}

/// Extract an IPv4 address from `ip addr` output or a bare `getprop` value.
fn parse_ip(text: &str) -> Option<String> {
    for line in text.lines() {
        let mut tokens = line.split_whitespace();
        while let Some(token) = tokens.next() {
            if token != "inet" {
                continue;
            }
            let candidate = tokens.next().and_then(|value| value.split('/').next());
            if let Some(addr) = candidate.filter(|addr| addr.parse::<std::net::Ipv4Addr>().is_ok())
            {
                return Some(addr.to_string());
            }
        }
    }

    let trimmed = text.trim();
    if trimmed.parse::<std::net::Ipv4Addr>().is_ok() {
        return Some(trimmed.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_wellformed_payload() {
        let qr = generate();
        let (name, password) = parse_qr(&qr.payload).unwrap();
        assert_eq!(qr.password, password);
        assert!(name.starts_with("zesp-"));
        assert_eq!(qr.payload, format!("WIFI:T:ADB;S:{name};P:{password};;"));
    }

    #[test]
    fn rejects_malformed_payloads() {
        assert!(parse_qr("WIFI:T:ADB;;").is_err());
        assert!(parse_qr("WIFI:T:ADB;S:;P:123456;;").is_err());
        assert!(parse_qr("WIFI:T:ADB;S:ok;P:notdigits;;").is_err());
        assert!(parse_qr("WIFI:T:ADB;S:ok;P:123;;").is_err());
    }

    #[test]
    fn parses_mdns_output() {
        let sample = "\
List of discovered mdns services
adb-14141FDF600081 _adb._tcp 192.168.86.38:5555
studio-abcdEFghij _adb-tls-pairing._tcp 192.168.86.39:55861
adb-14141FDF600081-TnSdi9 _adb-tls-connect._tcp 192.168.86.38:33015
";
        let services = parse_mdns(sample);
        assert_eq!(services.len(), 3);
        assert_eq!(services[1].name, "studio-abcdEFghij");
        assert_eq!(services[1].service_type, PAIRING_SERVICE);
        assert_eq!(services[1].host, "192.168.86.39");
        assert_eq!(services[1].port, 55861);
    }

    #[test]
    fn ignores_junk_lines() {
        assert!(parse_mdns("List of discovered mdns services\n\nnot a service\n").is_empty());
    }

    #[test]
    fn parses_mdns_fixture() {
        let services = parse_mdns(include_str!("../../tests/fixtures/adb_mdns_services.txt"));
        assert_eq!(services.len(), 3);
        assert_eq!(services[0].service_type, CONNECT_SERVICE);
        assert_eq!(services[0].host, "192.168.1.42");
        assert_eq!(services[0].port, 5555);
        assert_eq!(services[1].service_type, PAIRING_SERVICE);
        assert_eq!(services[1].port, 37401);
    }

    #[test]
    fn recognises_connect_output() {
        assert!(connected(
            include_str!("../../tests/fixtures/adb_connect.txt").trim()
        ));
        assert!(connected("already connected to 192.168.1.42:5555"));
        assert!(!connected(
            "failed to connect to 192.168.1.42:5555: Connection refused"
        ));
        assert!(!connected("cannot connect to 192.168.1.42:5555"));
    }

    #[test]
    fn recognises_tcpip_output() {
        assert!(tcpip_enabled(
            include_str!("../../tests/fixtures/adb_tcpip.txt").trim()
        ));
        assert!(!tcpip_enabled("error: device 'X' not found"));
    }

    #[test]
    fn recognises_pair_output() {
        assert!(paired(
            include_str!("../../tests/fixtures/adb_pair.txt").trim()
        ));
        assert!(!paired("failed to pair to 192.168.1.42:37401"));
    }

    #[test]
    fn parses_wifi_ip() {
        let wlan0 = "3: wlan0: <BROADCAST,MULTICAST,UP> mtu 1500\n    inet 192.168.1.50/24 brd 192.168.1.255 scope global wlan0\n    inet6 fe80::1/64 scope link\n";
        assert_eq!(parse_ip(wlan0), Some("192.168.1.50".to_string()));
        assert_eq!(
            parse_ip("192.168.1.50"),
            Some("192.168.1.50".to_string()),
            "getprop fallback returns a bare address"
        );
        assert_eq!(parse_ip("ipv6 only"), None);
    }

    fn service(name: &str, host: &str, port: u16) -> MdnsService {
        MdnsService {
            name: name.into(),
            service_type: CONNECT_SERVICE.into(),
            host: host.into(),
            port,
        }
    }

    fn target(serial: &str, name: Option<&str>, address: Option<&str>) -> ReconnectTarget {
        ReconnectTarget {
            serial: serial.into(),
            name: name.map(Into::into),
            address: address.map(Into::into),
        }
    }

    fn wifi_device(serial: &str, address: Option<&str>, connected: bool) -> Device {
        let mut device = crate::services::devices::parse_devices(&format!(
            "{serial} {} model:Pixel_8",
            if connected { "device" } else { "offline" }
        ))
        .remove(0);
        device.address = address.map(Into::into);
        device
    }

    #[test]
    fn locates_a_phone_on_its_new_port_by_name_then_by_ip() {
        let services = vec![
            service("adb-OTHER-a", "192.168.1.42", 41000),
            service("adb-PIXEL-b", "192.168.1.60", 39123),
        ];
        let moved = target(
            "adb-PIXEL-b._adb-tls-connect._tcp",
            Some("adb-PIXEL-b"),
            Some("192.168.1.42:37000"),
        );
        assert_eq!(locate(&services, &moved).unwrap().port, 39123);

        let legacy = target("192.168.1.42:37000", None, Some("192.168.1.42:37000"));
        assert_eq!(locate(&services, &legacy).unwrap().name, "adb-OTHER-a");

        let gone = target("x", Some("adb-GONE-c"), None);
        assert!(locate(&services, &gone).is_none());
    }

    #[test]
    fn a_phone_is_online_through_any_of_its_transports() {
        let saved = target(
            "adb-PIXEL-b._adb-tls-connect._tcp",
            Some("adb-PIXEL-b"),
            None,
        );
        let found = service("adb-PIXEL-b", "192.168.1.60", 39123);

        let by_ip = [wifi_device(
            "192.168.1.60:39123",
            Some("192.168.1.60:39123"),
            true,
        )];
        assert!(is_online(&by_ip, &saved, Some(&found)));

        let by_alias = [wifi_device("adb-PIXEL-b._adb-tls-connect._tcp", None, true)];
        assert!(is_online(&by_alias, &saved, None));

        let stale = [wifi_device(
            "192.168.1.60:37000",
            Some("192.168.1.60:37000"),
            false,
        )];
        assert!(!is_online(&stale, &saved, Some(&found)));
    }

    #[test]
    fn finds_only_dead_transports_on_old_ports() {
        let output = "List of devices attached
192.168.1.60:37000	offline
192.168.1.60:39123	device
192.168.1.42:37000	offline
adb-PIXEL-b._adb-tls-connect._tcp	device
";
        assert_eq!(
            stale_serials(output, "192.168.1.60", "192.168.1.60:39123"),
            vec!["192.168.1.60:37000"]
        );

        let found = service("adb-PIXEL-b", "192.168.1.60", 39123);
        let old = [wifi_device(
            "adb-PIXEL-b._adb-tls-connect._tcp",
            Some("192.168.1.60:37000"),
            true,
        )];
        assert!(has_stale_port(&old, &found));
        let current = [wifi_device(
            "192.168.1.60:39123",
            Some("192.168.1.60:39123"),
            true,
        )];
        assert!(!has_stale_port(&current, &found));
    }

    #[test]
    fn classifies_connect_results() {
        let target = || "192.168.1.60:39123".to_string();
        assert_eq!(
            classify_connect("connected to 192.168.1.60:39123", target()),
            Reconnect::Connected(target())
        );
        assert_eq!(
            classify_connect("failed to authenticate to 192.168.1.60:39123", target()),
            Reconnect::Unpaired
        );
        assert!(matches!(
            classify_connect(
                "failed to connect to 192.168.1.60:39123: Connection refused",
                target()
            ),
            Reconnect::Unavailable(_)
        ));
    }
}
