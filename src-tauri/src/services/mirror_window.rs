//! Client-area sizing for in-app mirror windows.
//!
//! Tauri/tao expose no aspect-ratio lock, so a mirror window keeps the video's
//! shape by hand. The first decoded frame reveals the device's dimensions and we
//! size the window to them ([`fit_size`]), so the client area matches the frame
//! and the canvas fills the window instead of being letterboxed with padding —
//! a landscape app gets a landscape window automatically.
//!
//! Keeping that shape while the user drags an edge is done two ways. On Windows
//! the native `WM_SIZING` hook asks us for the rectangle *before* it is applied,
//! and [`resize`] answers with one on the video's aspect ratio, anchoring the
//! opposite edge/corner so the dragged one tracks the cursor. `WM_SIZING` gives
//! the *outer* rectangle, hence [`Frame`] (the non-client border/title size) is
//! subtracted to work in client space. Where there is no such hook the caller
//! falls back to correcting the size afterwards with [`lock_size`].

/// Smallest client-area side a mirror window may shrink to. The window's minimum
/// size is set to this in both dimensions so the OS never clamps a resize in a way
/// that would fight the aspect ratio.
pub const MIN_SIDE: f64 = 240.0;

/// The longer side a window takes the first time it fits a session, and the
/// footprint a freshly opened mirror starts at.
pub const DEFAULT_LONG_SIDE: f64 = 820.0;

/// Upper bound on the longer side kept across a rotation, so a window the user
/// stretched very wide cannot produce an absurd size on the next turn.
pub const MAX_LONG_SIDE: f64 = 2000.0;

/// Grow `(width, height)` — staying on `aspect = width / height` — until neither
/// side is below [`MIN_SIDE`], rounding to whole pixels for stable `set_size`s.
fn with_minimum(width: f64, height: f64, aspect: f64) -> (f64, f64) {
    let (mut width, mut height) = (width, height);
    if width < MIN_SIDE {
        width = MIN_SIDE;
        height = width / aspect;
    }
    if height < MIN_SIDE {
        height = MIN_SIDE;
        width = height * aspect;
    }
    (width.round(), height.round())
}

/// The window size that shows a `width`×`height` video with its longer side at
/// `long_side`, preserving the video's aspect ratio.
pub fn fit_size(width: u32, height: u32, long_side: f64) -> (f64, f64) {
    let aspect = width as f64 / height as f64;
    let long_side = long_side.clamp(MIN_SIDE * 2.0, MAX_LONG_SIDE);
    if aspect >= 1.0 {
        with_minimum(long_side, long_side / aspect, aspect)
    } else {
        with_minimum(long_side * aspect, long_side, aspect)
    }
}

/// Correct a size the user dragged to back onto `aspect`, keeping the width they
/// chose and letting the height follow. Used where the platform offers no resize
/// hook to constrain the rectangle up front.
#[cfg_attr(windows, allow(dead_code))]
pub fn lock_size(width: f64, aspect: f64) -> (f64, f64) {
    let width = width.max(MIN_SIDE);
    with_minimum(width, width / aspect, aspect)
}

/// A window rectangle in physical pixels, matching the Win32 `RECT` used by
/// `WM_SIZING`.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// The non-client size of a window: the border, title bar and shadows the OS adds
/// around the client area. `WM_SIZING` deals in the outer rectangle, so the two
/// are subtracted before the aspect ratio is applied.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Frame {
    pub width: i32,
    pub height: i32,
}

/// Which edge or corner the user is dragging, from the `WM_SIZING` `wParam`.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// Scale a client size up, preserving the ratio, until neither side is below
/// [`MIN_SIDE`].
fn grow_to_minimum(width: f64, height: f64) -> (f64, f64) {
    let scale = (MIN_SIDE / width).max(MIN_SIDE / height).max(1.0);
    ((width * scale).round(), (height * scale).round())
}

/// Force a dragged outer rectangle back onto the client aspect ratio, anchoring
/// the edge/corner opposite the one being dragged so the dragged one follows the
/// cursor.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn resize(mut rect: Rect, aspect: f64, frame: Frame, edge: Edge) -> Rect {
    let (anchor_x, anchor_y) = anchor(rect, edge);
    let (dragged_x, dragged_y) = dragged(rect, edge);

    // The client size the cursor is asking for, given the fixed anchor.
    let outer_width = (dragged_x - anchor_x).unsigned_abs() as f64;
    let outer_height = (dragged_y - anchor_y).unsigned_abs() as f64;
    let client_width = (outer_width - f64::from(frame.width)).max(1.0);
    let client_height = (outer_height - f64::from(frame.height)).max(1.0);

    let (width, height) = match edge {
        // A side only drives one dimension; the ratio supplies the other.
        Edge::Left | Edge::Right => (client_width, client_width / aspect),
        Edge::Top | Edge::Bottom => (client_height * aspect, client_height),
        // A corner drives whichever dimension is further from the ratio, so the
        // rectangle encloses the cursor instead of jumping past it.
        _ => {
            if client_width / client_height >= aspect {
                (client_width, client_width / aspect)
            } else {
                (client_height * aspect, client_height)
            }
        }
    };
    let (width, height) = grow_to_minimum(width, height);
    let (width, height) = (width as i32, height as i32);
    // Back to outer coordinates, then re-apply the anchor.
    let (width, height) = (width + frame.width, height + frame.height);

    match edge {
        Edge::Right | Edge::BottomRight | Edge::TopRight => rect.right = anchor_x + width,
        Edge::Left | Edge::TopLeft | Edge::BottomLeft => rect.left = anchor_x - width,
        _ => {}
    }
    match edge {
        Edge::Bottom | Edge::BottomRight | Edge::BottomLeft => rect.bottom = anchor_y + height,
        Edge::Top | Edge::TopLeft | Edge::TopRight => rect.top = anchor_y - height,
        _ => {}
    }
    // Sides that do not move keep both perpendicular coordinates, but the derived
    // dimension still needs applying (e.g. a right-edge drag changes the height).
    match edge {
        Edge::Left | Edge::Right => rect.bottom = rect.top + height,
        Edge::Top | Edge::Bottom => rect.right = rect.left + width,
        _ => {}
    }
    rect
}

/// The fixed corner/edge of `rect` for the edge being dragged.
fn anchor(rect: Rect, edge: Edge) -> (i32, i32) {
    match edge {
        Edge::Left => (rect.right, rect.top),
        Edge::Right => (rect.left, rect.top),
        Edge::Top => (rect.right, rect.bottom),
        Edge::Bottom => (rect.left, rect.top),
        Edge::BottomRight => (rect.left, rect.top),
        Edge::TopLeft => (rect.right, rect.bottom),
        Edge::TopRight => (rect.left, rect.bottom),
        Edge::BottomLeft => (rect.right, rect.top),
    }
}

/// The corner/edge of `rect` the cursor is moving.
fn dragged(rect: Rect, edge: Edge) -> (i32, i32) {
    match edge {
        Edge::Left => (rect.left, rect.top),
        Edge::Right => (rect.right, rect.top),
        Edge::Top => (rect.right, rect.top),
        Edge::Bottom => (rect.left, rect.bottom),
        Edge::BottomRight => (rect.right, rect.bottom),
        Edge::TopLeft => (rect.left, rect.top),
        Edge::TopRight => (rect.right, rect.top),
        Edge::BottomLeft => (rect.left, rect.bottom),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_a_portrait_video_to_the_long_side() {
        assert_eq!(fit_size(1080, 2400, DEFAULT_LONG_SIDE), (369.0, 820.0));
    }

    #[test]
    fn fits_a_landscape_video_to_the_long_side() {
        assert_eq!(fit_size(2400, 1080, DEFAULT_LONG_SIDE), (820.0, 369.0));
    }

    #[test]
    fn keeps_the_ratio_when_fitting() {
        let (w, h) = fit_size(1080, 2340, 900.0);
        let ratio = w / h;
        assert!(
            (ratio - 1080.0 / 2340.0).abs() < 0.01,
            "ratio drifted: {ratio}"
        );
    }

    #[test]
    fn locks_the_height_to_the_width() {
        // 0.45 (a 1080x2400 phone) keeps a dragged width of 500 and follows to 1111.
        assert_eq!(lock_size(500.0, 0.45), (500.0, 1111.0));
    }

    #[test]
    fn lock_grows_a_too_narrow_width_back_to_the_minimum() {
        // width 100 on a 2:1 landscape would put the height under the minimum, so
        // both sides scale up to satisfy it.
        assert_eq!(lock_size(100.0, 2.0), (480.0, 240.0));
    }

    #[test]
    fn caps_the_long_side() {
        let (w, h) = fit_size(2400, 1080, 10_000.0);
        assert_eq!(w, MAX_LONG_SIDE);
        assert_eq!(h, MAX_LONG_SIDE * 1080.0 / 2400.0);
    }

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    const NO_FRAME: Frame = Frame {
        width: 0,
        height: 0,
    };

    #[test]
    fn keeps_a_corner_drag_on_the_ratio() {
        // A 400x800 client is already 0.5, so a bottom-right drag is a no-op.
        let start = rect(0, 0, 400, 800);
        assert_eq!(resize(start, 0.5, NO_FRAME, Edge::BottomRight), start);
    }

    #[test]
    fn a_wider_corner_drag_drives_the_width() {
        // 500/800 = 0.625 >= 0.5, so the width wins and the height follows.
        assert_eq!(
            resize(rect(0, 0, 500, 800), 0.5, NO_FRAME, Edge::BottomRight),
            rect(0, 0, 500, 1000)
        );
    }

    #[test]
    fn a_taller_corner_drag_drives_the_height() {
        // 300/900 = 0.333 < 0.5, so the height wins and the width follows.
        assert_eq!(
            resize(rect(0, 0, 300, 900), 0.5, NO_FRAME, Edge::BottomRight),
            rect(0, 0, 450, 900)
        );
    }

    #[test]
    fn anchors_the_corner_opposite_the_drag() {
        // Dragging the top-left keeps the bottom-right pinned.
        let start = rect(100, 100, 500, 900);
        assert_eq!(resize(start, 0.5, NO_FRAME, Edge::TopLeft), start);
    }

    #[test]
    fn a_side_drag_derives_the_other_side() {
        assert_eq!(
            resize(rect(0, 0, 600, 200), 0.5, NO_FRAME, Edge::Right),
            rect(0, 0, 600, 1200)
        );
        assert_eq!(
            resize(rect(0, 0, 200, 600), 0.5, NO_FRAME, Edge::Bottom),
            rect(0, 0, 300, 600)
        );
    }

    #[test]
    fn subtracts_the_window_frame_before_applying_the_ratio() {
        // Outer 416x839 with a 16x39 frame is a 400x800 client, already on 0.5.
        let start = rect(0, 0, 416, 839);
        assert_eq!(
            resize(
                start,
                0.5,
                Frame {
                    width: 16,
                    height: 39
                },
                Edge::BottomRight
            ),
            start
        );
    }

    #[test]
    fn grows_a_corner_drag_back_to_the_minimum() {
        assert_eq!(
            resize(rect(0, 0, 100, 100), 1.0, NO_FRAME, Edge::BottomRight),
            rect(0, 0, 240, 240)
        );
    }

    #[test]
    fn never_goes_below_the_minimum_side() {
        let (w, h) = fit_size(1080, 2400, 10.0);
        assert!(
            w >= MIN_SIDE && h >= MIN_SIDE,
            "{w}x{h} is below the minimum"
        );
        assert!((w / h - 1080.0 / 2400.0).abs() < 0.01);
    }
}
