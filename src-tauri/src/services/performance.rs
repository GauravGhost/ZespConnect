use std::process::Stdio;

use crate::error::{AppError, Result};
use crate::models::{ForegroundApp, PerfSample};
use crate::util::process;

/// Printed by the phone after each full snapshot.
pub const END_MARKER: &str = "::end";
const SECTION_PREFIX: &str = "::";
const INTERVAL_SECS: u32 = 1;

/// The phone-side loop. One `adb shell` runs for the whole session so the computer spawns no
/// process per sample; each pass only reads `/proc`, `/sys` and two short `dumpsys` calls.
fn watch_script() -> String {
    format!(
        "while :; do \
         echo ::uptime; cat /proc/uptime; \
         echo ::stat; grep '^cpu' /proc/stat; \
         echo ::mem; grep -E '^(MemTotal|MemAvailable):' /proc/meminfo; \
         echo ::battery; cat /sys/class/power_supply/battery/uevent 2>/dev/null; \
         echo ::net; cat /proc/net/dev 2>/dev/null; \
         f=$(dumpsys window displays 2>/dev/null | grep -m1 mCurrentFocus); p=${{f#* u[0-9]* }}; p=${{p%%/*}}; \
         echo ::focus; echo \"$p\"; \
         echo ::gfx; dumpsys gfxinfo \"$p\" 2>/dev/null | grep -m2 -E '^(Total frames rendered|Janky frames):'; \
         echo ::refresh; dumpsys SurfaceFlinger --latency 2>/dev/null; \
         echo {END_MARKER}; sleep {INTERVAL_SECS}; done"
    )
}

/// Start the phone-side loop; dropping the child ends it on the phone too.
pub fn spawn_watch(adb_path: &str, serial: &str) -> Result<tokio::process::Child> {
    let args = vec![
        "-s".to_string(),
        serial.to_string(),
        "shell".to_string(),
        watch_script(),
    ];
    process::base_command(adb_path, &args)
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| AppError::CommandFailed(format!("Could not start adb shell: {e}")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CpuTimes {
    busy: u64,
    total: u64,
}

/// Raw counters from one pass of the phone loop, before deltas turn them into rates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Snapshot {
    uptime_secs: Option<f64>,
    cpu: Option<CpuTimes>,
    cores: Vec<(usize, CpuTimes)>,
    mem_total_kb: Option<u64>,
    mem_available_kb: Option<u64>,
    battery_level: Option<u8>,
    battery_temp_tenths: Option<i64>,
    battery_current_raw: Option<i64>,
    battery_voltage_raw: Option<i64>,
    battery_status: Option<String>,
    net_bytes: Option<(u64, u64)>,
    package: Option<String>,
    frames: Option<(u64, u64)>,
    refresh_hz: Option<f32>,
}

pub fn parse_snapshot(text: &str) -> Snapshot {
    let mut snapshot = Snapshot::default();
    let mut section = "";
    for line in text.lines() {
        let line = line.trim();
        if let Some(name) = line.strip_prefix(SECTION_PREFIX) {
            section = name;
            continue;
        }
        if line.is_empty() {
            continue;
        }
        match section {
            "uptime" => {
                snapshot.uptime_secs = line.split_whitespace().next().and_then(|v| v.parse().ok())
            }
            "stat" => parse_cpu_line(line, &mut snapshot),
            "mem" => parse_mem_line(line, &mut snapshot),
            "battery" => parse_battery_line(line, &mut snapshot),
            "net" => parse_net_line(line, &mut snapshot),
            "focus" => snapshot.package = valid_package(line),
            "gfx" => parse_gfx_line(line, &mut snapshot),
            "refresh" => {
                snapshot.refresh_hz = line
                    .parse::<f64>()
                    .ok()
                    .filter(|period_ns| *period_ns > 0.0)
                    .map(|period_ns| (1e9 / period_ns) as f32)
            }
            _ => {}
        }
    }
    snapshot
}

fn parse_cpu_line(line: &str, snapshot: &mut Snapshot) {
    let mut fields = line.split_whitespace();
    let Some(name) = fields.next() else { return };
    let values: Vec<u64> = fields.filter_map(|v| v.parse().ok()).collect();
    // user nice system idle iowait irq softirq steal; idle and iowait are the resting time.
    if values.len() < 4 {
        return;
    }
    let total: u64 = values.iter().take(8).sum();
    let idle = values[3] + values.get(4).copied().unwrap_or(0);
    let times = CpuTimes {
        busy: total.saturating_sub(idle),
        total,
    };
    match name.strip_prefix("cpu") {
        Some("") => snapshot.cpu = Some(times),
        Some(index) => {
            if let Ok(index) = index.parse() {
                snapshot.cores.push((index, times));
            }
        }
        None => {}
    }
}

fn parse_mem_line(line: &str, snapshot: &mut Snapshot) {
    let Some((key, rest)) = line.split_once(':') else {
        return;
    };
    let value = rest.split_whitespace().next().and_then(|v| v.parse().ok());
    match key {
        "MemTotal" => snapshot.mem_total_kb = value,
        "MemAvailable" => snapshot.mem_available_kb = value,
        _ => {}
    }
}

fn parse_battery_line(line: &str, snapshot: &mut Snapshot) {
    let Some((key, value)) = line.split_once('=') else {
        return;
    };
    match key {
        "POWER_SUPPLY_CAPACITY" => snapshot.battery_level = value.parse().ok(),
        "POWER_SUPPLY_TEMP" => snapshot.battery_temp_tenths = value.parse().ok(),
        "POWER_SUPPLY_CURRENT_NOW" => snapshot.battery_current_raw = value.parse().ok(),
        "POWER_SUPPLY_VOLTAGE_NOW" => snapshot.battery_voltage_raw = value.parse().ok(),
        "POWER_SUPPLY_STATUS" => snapshot.battery_status = Some(value.to_string()),
        _ => {}
    }
}

/// Interfaces that loop back or mirror another one's traffic, so counting them would double it.
const SKIPPED_INTERFACES: [&str; 4] = ["lo", "dummy", "ifb", "rmnet_ipa"];

/// Sum the real interfaces: `name: rx_bytes ... (8 rx fields) tx_bytes ...`.
fn parse_net_line(line: &str, snapshot: &mut Snapshot) {
    let Some((name, counters)) = line.split_once(':') else {
        return;
    };
    let name = name.trim();
    if SKIPPED_INTERFACES.iter().any(|skip| name.starts_with(skip)) {
        return;
    }
    let values: Vec<u64> = counters
        .split_whitespace()
        .filter_map(|v| v.parse().ok())
        .collect();
    let (Some(rx), Some(tx)) = (values.first(), values.get(8)) else {
        return;
    };
    let (total_rx, total_tx) = snapshot.net_bytes.unwrap_or((0, 0));
    snapshot.net_bytes = Some((total_rx + rx, total_tx + tx));
}

fn parse_gfx_line(line: &str, snapshot: &mut Snapshot) {
    let Some((key, rest)) = line.split_once(':') else {
        return;
    };
    let Some(value) = rest.split_whitespace().next().and_then(|v| v.parse().ok()) else {
        return;
    };
    let (total, janky) = snapshot.frames.unwrap_or((0, 0));
    snapshot.frames = match key {
        "Total frames rendered" => Some((value, janky)),
        "Janky frames" => Some((total, value)),
        _ => snapshot.frames,
    };
}

/// The focus line is a window name when no app is focused (lock screen, shade), not a package.
fn valid_package(text: &str) -> Option<String> {
    crate::util::validation::validate_package(text)
        .is_ok()
        .then(|| text.to_string())
}

/// Turns consecutive snapshots into rates; the first snapshot only yields the absolute values.
#[derive(Default)]
pub struct Sampler {
    previous: Option<Snapshot>,
}

impl Sampler {
    pub fn sample(&mut self, snapshot: Snapshot, label: impl Fn(&str) -> String) -> PerfSample {
        let previous = self.previous.as_ref();
        let elapsed = previous
            .and_then(|p| Some(snapshot.uptime_secs? - p.uptime_secs?))
            .filter(|secs| *secs > 0.0);

        let cpu = previous.and_then(|p| busy_percent(p.cpu?, snapshot.cpu?));
        let core_count = snapshot.cores.iter().map(|(i, _)| i + 1).max().unwrap_or(0);
        let cores = (0..core_count)
            .map(|index| {
                let now = core(&snapshot, index)?;
                busy_percent(core(previous?, index)?, now)
            })
            .collect();

        let same_app =
            previous.is_some_and(|p| p.package.is_some() && p.package == snapshot.package);
        let frame_delta = previous
            .filter(|_| same_app)
            .and_then(|p| Some((p.frames?, snapshot.frames?)))
            // The counters restart with the app's process; a drop is not negative frames.
            .filter(|(before, now)| now.0 >= before.0 && now.1 >= before.1)
            .map(|(before, now)| (now.0 - before.0, now.1 - before.1));
        let fps = frame_delta
            .zip(elapsed)
            .map(|((frames, _), secs)| (frames as f64 / secs) as f32);
        let jank = frame_delta
            .filter(|(frames, _)| *frames > 0)
            .map(|(frames, janky)| (janky as f32 / frames as f32 * 100.0).min(100.0));

        let net = previous
            .and_then(|p| Some((p.net_bytes?, snapshot.net_bytes?)))
            .zip(elapsed)
            .map(|((before, now), secs)| {
                let rate = |a: u64, b: u64| (b.saturating_sub(a) as f64 / secs) as u64;
                (rate(before.0, now.0), rate(before.1, now.1))
            });

        let battery_ma = snapshot.battery_current_raw.map(current_ma);
        let battery_volts = snapshot.battery_voltage_raw.map(voltage_volts);
        let sample = PerfSample {
            cpu,
            cores,
            mem_used_kb: snapshot
                .mem_total_kb
                .zip(snapshot.mem_available_kb)
                .map(|(total, available)| total.saturating_sub(available)),
            mem_total_kb: snapshot.mem_total_kb,
            fps,
            jank,
            refresh_hz: snapshot.refresh_hz,
            foreground: snapshot.package.as_ref().map(|package| ForegroundApp {
                label: label(package),
                package: package.clone(),
            }),
            battery_level: snapshot.battery_level,
            battery_temp_c: snapshot.battery_temp_tenths.map(|t| t as f32 / 10.0),
            battery_ma: battery_ma.map(f32::abs),
            battery_watts: battery_ma
                .zip(battery_volts)
                .map(|(ma, volts)| ma.abs() / 1000.0 * volts),
            charging: snapshot
                .battery_status
                .as_deref()
                .map(|status| status == "Charging"),
            net_rx_bps: net.map(|(rx, _)| rx),
            net_tx_bps: net.map(|(_, tx)| tx),
        };
        self.previous = Some(snapshot);
        sample
    }
}

fn core(snapshot: &Snapshot, index: usize) -> Option<CpuTimes> {
    snapshot
        .cores
        .iter()
        .find(|(i, _)| *i == index)
        .map(|(_, times)| *times)
}

fn busy_percent(before: CpuTimes, now: CpuTimes) -> Option<f32> {
    let total = now.total.checked_sub(before.total).filter(|t| *t > 0)?;
    let busy = now.busy.saturating_sub(before.busy).min(total);
    Some(busy as f32 / total as f32 * 100.0)
}

/// Most kernels report microamps, some (older Samsung) milliamps; no phone draws 20 A.
fn current_ma(raw: i64) -> f32 {
    if raw.abs() > 20_000 {
        raw as f32 / 1000.0
    } else {
        raw as f32
    }
}

/// Microvolts on most kernels, millivolts on a few.
fn voltage_volts(raw: i64) -> f32 {
    if raw > 100_000 {
        raw as f32 / 1_000_000.0
    } else {
        raw as f32 / 1000.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(uptime: &str, cpu: &str, core0: &str, frames: u64, janky: u64, rx: u64) -> String {
        format!(
            "::uptime\n{uptime} 100.0\n::stat\ncpu  {cpu}\ncpu0 {core0}\n\
             ::mem\nMemTotal:        8000000 kB\nMemAvailable:    2000000 kB\n\
             ::battery\nPOWER_SUPPLY_STATUS=Discharging\nPOWER_SUPPLY_CAPACITY=36\n\
             POWER_SUPPLY_TEMP=290\nPOWER_SUPPLY_CURRENT_NOW=-960312\nPOWER_SUPPLY_VOLTAGE_NOW=3670312\n\
             ::net\nInter-|   Receive\n face |bytes    packets\n    lo: 999 1 0 0 0 0 0 0 999 1 0 0 0 0 0 0\n\
             wlan0: {rx} 10 0 0 0 0 0 0 500 5 0 0 0 0 0 0\n\
             ::focus\ncom.example.app\n\
             ::gfx\nTotal frames rendered: {frames}\nJanky frames: {janky} (1.90%)\n\
             ::refresh\n16666667\n::end\n"
        )
    }

    #[test]
    fn parses_absolute_values() {
        let snapshot = parse_snapshot(&pass(
            "10.0",
            "10 0 10 80 0 0 0 0",
            "5 0 5 40 0 0 0 0",
            100,
            2,
            1000,
        ));
        assert_eq!(snapshot.uptime_secs, Some(10.0));
        assert_eq!(
            snapshot.cpu,
            Some(CpuTimes {
                busy: 20,
                total: 100
            })
        );
        assert_eq!(snapshot.mem_available_kb, Some(2_000_000));
        assert_eq!(snapshot.net_bytes, Some((1000, 500)));
        assert_eq!(snapshot.package.as_deref(), Some("com.example.app"));
        assert_eq!(snapshot.frames, Some((100, 2)));
        assert_eq!(snapshot.refresh_hz.map(f32::round), Some(60.0));
    }

    #[test]
    fn first_sample_has_no_rates() {
        let mut sampler = Sampler::default();
        let snapshot = parse_snapshot(&pass(
            "10.0",
            "10 0 10 80 0 0 0 0",
            "5 0 5 40 0 0 0 0",
            100,
            2,
            1000,
        ));
        let sample = sampler.sample(snapshot, str::to_string);
        assert_eq!(sample.cpu, None);
        assert_eq!(sample.fps, None);
        assert_eq!(sample.mem_used_kb, Some(6_000_000));
        assert_eq!(sample.battery_temp_c, Some(29.0));
        assert!((sample.battery_ma.unwrap() - 960.312).abs() < 0.01);
        assert!((sample.battery_watts.unwrap() - 3.524).abs() < 0.01);
        assert_eq!(sample.charging, Some(false));
    }

    #[test]
    fn second_sample_turns_counters_into_rates() {
        let mut sampler = Sampler::default();
        let first = parse_snapshot(&pass(
            "10.0",
            "10 0 10 80 0 0 0 0",
            "5 0 5 40 0 0 0 0",
            100,
            2,
            1000,
        ));
        let second = parse_snapshot(&pass(
            "12.0",
            "40 0 30 130 0 0 0 0",
            "20 0 10 70 0 0 0 0",
            220,
            8,
            5000,
        ));
        sampler.sample(first, str::to_string);
        let sample = sampler.sample(second, str::to_string);
        assert_eq!(sample.cpu, Some(50.0));
        assert_eq!(sample.cores, vec![Some(40.0)]);
        assert_eq!(sample.fps, Some(60.0));
        assert_eq!(sample.jank, Some(5.0));
        assert_eq!(sample.net_rx_bps, Some(2000));
        assert_eq!(sample.net_tx_bps, Some(0));
    }

    #[test]
    fn restarted_app_does_not_report_negative_frames() {
        let mut sampler = Sampler::default();
        let first = parse_snapshot(&pass(
            "10.0",
            "1 0 1 8 0 0 0 0",
            "1 0 1 8 0 0 0 0",
            500,
            9,
            0,
        ));
        let second = parse_snapshot(&pass(
            "11.0",
            "2 0 2 16 0 0 0 0",
            "2 0 2 16 0 0 0 0",
            10,
            0,
            0,
        ));
        sampler.sample(first, str::to_string);
        assert_eq!(sampler.sample(second, str::to_string).fps, None);
    }

    #[test]
    fn window_names_are_not_packages() {
        assert_eq!(valid_package("NotificationShade}"), None);
        assert_eq!(valid_package(""), None);
    }

    #[test]
    fn current_units_are_normalised() {
        assert_eq!(current_ma(-960_312), -960.312);
        assert_eq!(current_ma(450), 450.0);
        assert!((voltage_volts(3_670_312) - 3.670312).abs() < 1e-6);
        assert!((voltage_volts(3_870) - 3.87).abs() < 1e-6);
    }
}
