//! Pure rules for the app launcher popup: which phone it acts on and which apps it lists.

use crate::models::{AppInfo, Device, LabelSource, LauncherTarget};

/// The selected phone when it is known, else the only connected phone.
pub fn resolve_target(selected: Option<&str>, devices: &[Device]) -> LauncherTarget {
    if let Some(device) = selected.and_then(|serial| devices.iter().find(|d| d.serial == serial)) {
        return if device.state.is_connected() {
            ready(device)
        } else {
            LauncherTarget::Offline {
                name: device.display_name.clone(),
            }
        };
    }

    let mut connected = devices.iter().filter(|d| d.state.is_connected());
    match (connected.next(), connected.next()) {
        (Some(device), None) => ready(device),
        (Some(_), Some(_)) => LauncherTarget::PickInTray,
        (None, _) => LauncherTarget::NoPhone,
    }
}

fn ready(device: &Device) -> LauncherTarget {
    LauncherTarget::Ready {
        serial: device.serial.clone(),
        name: device.display_name.clone(),
    }
}

/// Apps worth offering, by label. `pm` lists every system package, launchable or not, so system
/// packages only it reported are dropped.
pub fn launchable(mut apps: Vec<AppInfo>) -> Vec<AppInfo> {
    apps.retain(|app| !(app.is_system && app.label_source == LabelSource::Derived));
    apps.sort_by_key(|app| app.label.to_lowercase());
    apps
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{DeviceState, DeviceTransport};

    fn device(serial: &str, state: DeviceState) -> Device {
        Device {
            serial: serial.to_string(),
            display_name: format!("Phone {serial}"),
            model: "Pixel".to_string(),
            android_version: None,
            sdk: None,
            transport: DeviceTransport::Usb,
            state,
            connected: state.is_connected(),
            battery: None,
            charging: None,
            address: None,
            mdns_name: None,
        }
    }

    fn app(label: &str, is_system: bool, label_source: LabelSource) -> AppInfo {
        AppInfo {
            package: format!("pkg.{}", label.to_lowercase()),
            label: label.to_string(),
            is_system,
            version_name: None,
            label_source,
            icon_path: None,
        }
    }

    fn ready(serial: &str) -> LauncherTarget {
        LauncherTarget::Ready {
            serial: serial.to_string(),
            name: format!("Phone {serial}"),
        }
    }

    #[test]
    fn selected_connected_device_is_ready() {
        let devices = [device("a", DeviceState::Device)];
        assert_eq!(resolve_target(Some("a"), &devices), ready("a"));
    }

    #[test]
    fn selected_offline_device_is_offline() {
        let devices = [device("a", DeviceState::Offline)];
        assert_eq!(
            resolve_target(Some("a"), &devices),
            LauncherTarget::Offline {
                name: "Phone a".to_string()
            }
        );
    }

    #[test]
    fn no_selection_one_connected_uses_it() {
        let devices = [
            device("a", DeviceState::Device),
            device("b", DeviceState::Offline),
        ];
        assert_eq!(resolve_target(None, &devices), ready("a"));
    }

    #[test]
    fn no_selection_two_connected_asks_for_tray() {
        let devices = [
            device("a", DeviceState::Device),
            device("b", DeviceState::Device),
        ];
        assert_eq!(resolve_target(None, &devices), LauncherTarget::PickInTray);
    }

    #[test]
    fn no_devices_is_no_phone() {
        assert_eq!(resolve_target(None, &[]), LauncherTarget::NoPhone);
    }

    #[test]
    fn forgotten_selection_falls_back() {
        let devices = [device("a", DeviceState::Device)];
        assert_eq!(resolve_target(Some("gone"), &devices), ready("a"));
    }

    #[test]
    fn launchable_hides_pm_system_packages() {
        let apps = vec![
            app("Background", true, LabelSource::Derived),
            app("Camera", true, LabelSource::Scrcpy),
            app("Notes", false, LabelSource::Derived),
        ];
        let labels: Vec<String> = launchable(apps).into_iter().map(|a| a.label).collect();
        assert_eq!(labels, ["Camera", "Notes"]);
    }

    #[test]
    fn launchable_sorts_by_label_case_insensitive() {
        let apps = vec![
            app("beta", false, LabelSource::Scrcpy),
            app("Alpha", false, LabelSource::Scrcpy),
        ];
        let labels: Vec<String> = launchable(apps).into_iter().map(|a| a.label).collect();
        assert_eq!(labels, ["Alpha", "beta"]);
    }
}
