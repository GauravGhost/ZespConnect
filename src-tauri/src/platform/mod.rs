//! Platform-specific process helpers.

#[cfg(windows)]
mod aspect;

/// Lock a mirror window's resizes to the video's aspect ratio through the
/// platform's own resize hook (Windows `WM_SIZING`), so the dragged edge tracks
/// the cursor. A no-op elsewhere, where the caller corrects the size afterwards.
#[cfg(windows)]
pub fn watch_aspect<R: tauri::Runtime>(window: &tauri::WebviewWindow<R>, aspect: f64) {
    let (Ok(inner), Ok(outer)) = (window.inner_size(), window.outer_size()) else {
        return;
    };
    let frame = crate::services::mirror_window::Frame {
        width: outer.width.saturating_sub(inner.width) as i32,
        height: outer.height.saturating_sub(inner.height) as i32,
    };
    if let Ok(hwnd) = window.hwnd() {
        aspect::watch(hwnd.0 as isize, aspect, frame);
    }
}

#[cfg(not(windows))]
pub fn watch_aspect<R: tauri::Runtime>(_window: &tauri::WebviewWindow<R>, _aspect: f64) {}

/// Stop locking a window as it closes, releasing the hook and its aspect.
#[cfg(windows)]
pub fn forget_aspect<R: tauri::Runtime>(window: &tauri::Window<R>) {
    if let Ok(hwnd) = window.hwnd() {
        aspect::forget(hwnd.0 as isize);
    }
}

#[cfg(not(windows))]
pub fn forget_aspect<R: tauri::Runtime>(_window: &tauri::Window<R>) {}

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

/// Rewrite a program + args so that Windows batch shims (`.cmd`/`.bat`) run via
/// `cmd.exe`, which cannot be spawned directly. Everything else is unchanged.
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
