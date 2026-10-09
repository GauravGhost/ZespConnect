//! Aspect-ratio locking for mirror windows on Windows. tao exposes neither a ratio lock nor a
//! `WM_SIZING` hook, so we subclass each window's procedure and clamp the dragged rectangle before
//! Windows applies it, keeping the dragged edge under the cursor.

use std::collections::HashMap;
use std::sync::Mutex;

use once_cell::sync::Lazy;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    WMSZ_BOTTOM, WMSZ_BOTTOMLEFT, WMSZ_BOTTOMRIGHT, WMSZ_LEFT, WMSZ_RIGHT, WMSZ_TOP, WMSZ_TOPLEFT,
    WMSZ_TOPRIGHT, WM_SIZING,
};

use crate::services::mirror_window::{self, Edge, Frame, Rect};

/// Identifies our subclass, so it can be removed without touching others'.
const SUBCLASS_ID: usize = 0x5A53_5001;

/// What to keep a window's client area on, keyed by its window handle.
#[derive(Clone, Copy)]
struct Target {
    aspect: f64,
    frame: Frame,
}

static TARGETS: Lazy<Mutex<HashMap<isize, Target>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Start locking `hwnd` to `aspect` for the given non-client `frame`; subclasses it only the first time.
pub fn watch(hwnd: isize, aspect: f64, frame: Frame) {
    let first = {
        let Ok(mut targets) = TARGETS.lock() else {
            return;
        };
        targets.insert(hwnd, Target { aspect, frame }).is_none()
    };
    if first {
        // SAFETY: `subclass_proc` is static and the handle is a live window of ours (removed by `forget`).
        unsafe {
            let _ = SetWindowSubclass(HWND(hwnd as *mut _), Some(subclass_proc), SUBCLASS_ID, 0);
        }
    }
}

/// Stop locking `hwnd` and forget its aspect (called as the window closes).
pub fn forget(hwnd: isize) {
    let known = TARGETS
        .lock()
        .map(|mut targets| targets.remove(&hwnd).is_some())
        .unwrap_or(false);
    if known {
        // SAFETY: symmetric with `watch`.
        unsafe {
            let _ = RemoveWindowSubclass(HWND(hwnd as *mut _), Some(subclass_proc), SUBCLASS_ID);
        }
    }
}

unsafe extern "system" fn subclass_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _data: usize,
) -> LRESULT {
    if message == WM_SIZING {
        if let Some(rect) = corrected_rect(hwnd, wparam, lparam) {
            // SAFETY: for WM_SIZING, lparam points at the rectangle being proposed.
            unsafe { *(lparam.0 as *mut RECT) = rect };
            // Handled: keep the rectangle we chose instead of the raw cursor one.
            return LRESULT(1);
        }
    }
    // SAFETY: forwards to the next procedure in the subclass chain (tao's).
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

/// The rectangle to apply for a `WM_SIZING` on the window's aspect ratio, or `None` if unknown.
fn corrected_rect(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) -> Option<RECT> {
    let target = TARGETS.lock().ok()?.get(&(hwnd.0 as isize)).copied()?;
    let edge = edge_from(wparam.0 as u32)?;

    // SAFETY: for WM_SIZING, lparam points at a RECT provided by the system.
    let rect = unsafe { &*(lparam.0 as *const RECT) };
    let corrected = mirror_window::resize(
        Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        },
        target.aspect,
        target.frame,
        edge,
    );
    Some(RECT {
        left: corrected.left,
        top: corrected.top,
        right: corrected.right,
        bottom: corrected.bottom,
    })
}

/// Map a `WM_SIZING` edge flag to the edge being dragged.
fn edge_from(flag: u32) -> Option<Edge> {
    Some(match flag {
        WMSZ_LEFT => Edge::Left,
        WMSZ_RIGHT => Edge::Right,
        WMSZ_TOP => Edge::Top,
        WMSZ_BOTTOM => Edge::Bottom,
        WMSZ_TOPLEFT => Edge::TopLeft,
        WMSZ_TOPRIGHT => Edge::TopRight,
        WMSZ_BOTTOMLEFT => Edge::BottomLeft,
        WMSZ_BOTTOMRIGHT => Edge::BottomRight,
        _ => return None,
    })
}
