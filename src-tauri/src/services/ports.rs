use crate::error::{AppError, Result};
use crate::models::{PortDirection, PortRule};
use crate::services::adb;
use crate::util::process::ProcessOutput;

/// Mirror and audio links tunnel through these; removing one breaks a running session.
const SCRCPY_SOCKET_PREFIX: &str = "localabstract:scrcpy";

/// `adb forward --list` covers every device as `<serial> <host> <device>`; keep this one's rules.
pub fn parse_forwards(text: &str, serial: &str) -> Vec<PortRule> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (owner, host, device) = (parts.next()?, parts.next()?, parts.next()?);
            (owner == serial).then(|| PortRule {
                direction: PortDirection::Forward,
                host: host.to_string(),
                device: device.to_string(),
            })
        })
        .collect()
}

/// `adb reverse --list` prints `<transport> <device> <host>`; the app's own scrcpy tunnels are hidden.
pub fn parse_reverses(text: &str) -> Vec<PortRule> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let (_, device, host) = (parts.next()?, parts.next()?, parts.next()?);
            Some(PortRule {
                direction: PortDirection::Reverse,
                host: host.to_string(),
                device: device.to_string(),
            })
        })
        .filter(|rule| !is_scrcpy_tunnel(rule))
        .collect()
}

fn is_scrcpy_tunnel(rule: &PortRule) -> bool {
    rule.direction == PortDirection::Reverse && rule.device.starts_with(SCRCPY_SOCKET_PREFIX)
}

fn validate_port(port: u16) -> Result<()> {
    if port == 0 {
        return Err(AppError::Invalid(
            "Ports must be between 1 and 65535".into(),
        ));
    }
    Ok(())
}

/// The socket adb identifies a rule by: the computer side for forwards, the phone side for reverses.
fn rule_key(rule: &PortRule) -> Result<&str> {
    let key = match rule.direction {
        PortDirection::Forward => rule.host.as_str(),
        PortDirection::Reverse => rule.device.as_str(),
    };
    let well_formed =
        key.contains(':') && !key.starts_with('-') && !key.chars().any(char::is_whitespace);
    if !well_formed || is_scrcpy_tunnel(rule) {
        return Err(AppError::Invalid(format!("Cannot remove port rule {key}")));
    }
    Ok(key)
}

fn subcommand(direction: PortDirection) -> String {
    match direction {
        PortDirection::Forward => "forward".into(),
        PortDirection::Reverse => "reverse".into(),
    }
}

fn ensure_success(output: ProcessOutput) -> Result<ProcessOutput> {
    if output.success {
        Ok(output)
    } else {
        Err(AppError::CommandFailed(output.text()))
    }
}

async fn run(adb_path: &str, serial: &str, args: Vec<String>) -> Result<ProcessOutput> {
    let output = adb::run_scoped(adb_path, serial, &args, adb::DEFAULT_TIMEOUT).await?;
    ensure_success(output)
}

pub async fn list(adb_path: &str, serial: &str) -> Result<Vec<PortRule>> {
    let list_args = |direction| vec![subcommand(direction), "--list".to_string()];
    let (forwards, reverses) = tokio::try_join!(
        run(adb_path, serial, list_args(PortDirection::Forward)),
        run(adb_path, serial, list_args(PortDirection::Reverse)),
    )?;
    let mut rules = parse_forwards(&forwards.stdout, serial);
    rules.extend(parse_reverses(&reverses.stdout));
    Ok(rules)
}

/// Add a tcp rule. `--no-rebind` turns a port already in use into an error instead of a silent swap.
pub async fn add(
    adb_path: &str,
    serial: &str,
    direction: PortDirection,
    host_port: u16,
    device_port: u16,
) -> Result<()> {
    validate_port(host_port)?;
    validate_port(device_port)?;
    let (host, device) = (format!("tcp:{host_port}"), format!("tcp:{device_port}"));
    let (from, to) = match direction {
        PortDirection::Forward => (host, device),
        PortDirection::Reverse => (device, host),
    };
    let args = vec![subcommand(direction), "--no-rebind".into(), from, to];
    run(adb_path, serial, args).await.map(drop)
}

pub async fn remove(adb_path: &str, serial: &str, rule: &PortRule) -> Result<()> {
    let key = rule_key(rule)?.to_string();
    let args = vec![subcommand(rule.direction), "--remove".into(), key];
    run(adb_path, serial, args).await.map(drop)
}

/// Remove every listed rule one by one; `--remove-all` would also drop the scrcpy tunnels.
pub async fn clear(adb_path: &str, serial: &str) -> Result<()> {
    for rule in list(adb_path, serial).await? {
        remove(adb_path, serial, &rule).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(direction: PortDirection, host: &str, device: &str) -> PortRule {
        PortRule {
            direction,
            host: host.into(),
            device: device.into(),
        }
    }

    #[test]
    fn keeps_only_this_devices_forwards() {
        let text = "192.168.1.121:42551 tcp:54056 tcp:9001\n\
                    emulator-5554 tcp:8000 tcp:8000\n\n";
        assert_eq!(
            parse_forwards(text, "192.168.1.121:42551"),
            vec![rule(PortDirection::Forward, "tcp:54056", "tcp:9001")]
        );
    }

    #[test]
    fn hides_scrcpy_reverse_tunnels() {
        let text = "host-36 localabstract:scrcpy_5d8568f2 tcp:53684\r\n\
                    host-35 tcp:3000 tcp:3001\r\n\r\n";
        assert_eq!(
            parse_reverses(text),
            vec![rule(PortDirection::Reverse, "tcp:3001", "tcp:3000")]
        );
    }

    #[test]
    fn keys_rules_by_the_listening_side() {
        let forward = rule(PortDirection::Forward, "tcp:1", "tcp:2");
        let reverse = rule(PortDirection::Reverse, "tcp:1", "tcp:2");
        assert_eq!(rule_key(&forward).unwrap(), "tcp:1");
        assert_eq!(rule_key(&reverse).unwrap(), "tcp:2");
    }

    #[test]
    fn refuses_unsafe_removals() {
        let scrcpy = rule(PortDirection::Reverse, "tcp:1", "localabstract:scrcpy_1");
        let option = rule(PortDirection::Forward, "--remove-all", "tcp:2");
        let spaced = rule(PortDirection::Forward, "tcp:1 tcp:2", "tcp:2");
        assert!(rule_key(&scrcpy).is_err());
        assert!(rule_key(&option).is_err());
        assert!(rule_key(&spaced).is_err());
    }
}
