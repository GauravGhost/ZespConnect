//! Platform-specific helpers: process spawning flags and macOS Dock visibility.

/// Hide the console window a spawned process would otherwise flash on Windows.
/// A no-op everywhere else.
#[cfg(windows)]
pub fn configure_command(command: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub fn configure_command(_command: &mut std::process::Command) {}

/// Rewrite a program + args so Windows batch shims (`.cmd`/`.bat`) run via `cmd.exe`; everything else is unchanged.
#[cfg(windows)]
pub fn prepare(program: &str, args: &[String]) -> (String, Vec<String>) {
    let lower = program.to_lowercase();
    if lower.ends_with(".cmd") || lower.ends_with(".bat") {
        let mut wrapped = Vec::with_capacity(args.len() + 2);
        wrapped.push("/c".to_string());
        wrapped.push(program.to_string());
        wrapped.extend_from_slice(args);
        ("cmd.exe".to_string(), wrapped)
    } else {
        (program.to_string(), args.to_vec())
    }
}

#[cfg(not(windows))]
pub fn prepare(program: &str, args: &[String]) -> (String, Vec<String>) {
    (program.to_string(), args.to_vec())
}

/// Executable suffix for the current platform.
pub const EXE_SUFFIX: &str = if cfg!(windows) { ".exe" } else { "" };

/// Give the app a Dock icon on macOS while it has a visible window. A no-op elsewhere.
#[cfg(target_os = "macos")]
pub fn show_in_dock(app: &tauri::AppHandle) {
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
}

#[cfg(not(target_os = "macos"))]
pub fn show_in_dock(_app: &tauri::AppHandle) {}

/// Run as a menu-bar-only app on macOS, without a Dock icon. A no-op elsewhere.
#[cfg(target_os = "macos")]
pub fn hide_from_dock(app: &tauri::AppHandle) {
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
}

#[cfg(not(target_os = "macos"))]
pub fn hide_from_dock(_app: &tauri::AppHandle) {}
