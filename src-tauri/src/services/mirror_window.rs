use crate::models::MirrorKind;

/// Smallest client-area side a mirror window may shrink to; set as the OS minimum in both dimensions.
pub const MIN_SIDE: f64 = 240.0;

/// The longer side a window takes the first time it fits a session.
pub const DEFAULT_LONG_SIDE: f64 = 820.0;

/// Height of the control bar below a phone window's video; `MirrorControlBar` is the same height (`h-9`).
pub const CONTROL_BAR: f64 = 36.0;

/// Margin below an app window's video, which has no control bar; `MirrorViewer` pads by the same (`pb-2`).
pub const BOTTOM_GAP: f64 = 8.0;

/// Height of a mirror window's own title bar, which replaces the native one on every platform.
pub const TITLE_BAR: f64 = 28.0;

/// The vertical space a window of `kind` reserves under and over its video: every window has a
/// title bar, a desktop nothing more, an app (Back is in its title bar) a small gap, and the rest
/// a `CONTROL_BAR`.
pub fn vertical_inset(kind: Option<MirrorKind>) -> f64 {
    match kind {
        Some(MirrorKind::Desktop) => TITLE_BAR,
        Some(MirrorKind::App) => TITLE_BAR + BOTTOM_GAP,
        _ => TITLE_BAR + CONTROL_BAR,
    }
}

/// The client size that holds a video area of `(width, height)`.
pub fn pad((width, height): (f64, f64), inset: f64) -> (f64, f64) {
    (width, height + inset)
}

/// The video area inside a client size of `(width, height)`.
pub fn unpad((width, height): (f64, f64), inset: f64) -> (f64, f64) {
    (width, (height - inset).max(1.0))
}

/// The display size that fills a window's video area for a display that follows the window
/// (`flex_display`), at `pixels_per_point` display pixels per logical window pixel. The window
/// arrives in physical pixels at `scale`; `None` while minimized or too small to hold any video.
pub fn display_pixels(
    (width, height): (u32, u32),
    scale: f64,
    inset: f64,
    pixels_per_point: f64,
) -> Option<(u16, u16)> {
    let positive = |value: f64| {
        if value.is_finite() && value > 0.0 {
            value
        } else {
            1.0
        }
    };
    let (scale, pixels_per_point) = (positive(scale), positive(pixels_per_point));
    let width = f64::from(width) / scale;
    let height = f64::from(height) / scale - inset;
    if width < 1.0 || height < 1.0 {
        return None;
    }
    let to_pixels = |value: f64| (value * pixels_per_point).round().min(f64::from(u16::MAX)) as u16;
    Some((to_pixels(width), to_pixels(height)))
}

/// Upper bound on the longer side across a rotation, so a stretched window cannot produce an absurd size.
pub const MAX_LONG_SIDE: f64 = 2000.0;

/// Grow `(width, height)` on `aspect` until neither side is below `MIN_SIDE`, rounded to whole pixels.
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

/// The window size that shows a `width`×`height` video with its longer side at `long_side`.
pub fn fit_size(width: u32, height: u32, long_side: f64) -> (f64, f64) {
    let aspect = width as f64 / height as f64;
    let long_side = long_side.clamp(MIN_SIDE * 2.0, MAX_LONG_SIDE);
    if aspect >= 1.0 {
        with_minimum(long_side, long_side / aspect, aspect)
    } else {
        with_minimum(long_side * aspect, long_side, aspect)
    }
}

/// Correct a dragged size back onto `aspect`, keeping the width and letting the height follow.
pub fn lock_size(width: f64, aspect: f64) -> (f64, f64) {
    let width = width.max(MIN_SIDE);
    with_minimum(width, width / aspect, aspect)
}

/// The client size that turns a window by a quarter turn: its video area swaps sides, so the long
/// side is unchanged and the inset is re-added below the video.
pub fn turned_size((width, height): (f64, f64), inset: f64) -> (f64, f64) {
    let (width, height) = unpad((width, height), inset);
    pad((height, width), inset)
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
    fn turns_a_window_by_a_quarter_turn() {
        // A 410x820 phone window with a 36 px inset keeps its 784 px long side when it turns.
        assert_eq!(turned_size((410.0, 820.0), 36.0), (784.0, 446.0));
        assert_eq!(turned_size((784.0, 446.0), 36.0), (410.0, 820.0));
    }

    #[test]
    fn pads_and_unpads_only_the_bottom() {
        assert_eq!(pad((100.0, 200.0), 10.0), (100.0, 210.0));
        assert_eq!(unpad(pad((100.0, 200.0), 10.0), 10.0), (100.0, 200.0));
        assert_eq!(unpad((4.0, 4.0), 10.0), (4.0, 1.0));
    }

    #[test]
    fn a_desktop_reserves_only_its_title_bar() {
        assert_eq!(vertical_inset(Some(MirrorKind::Desktop)), TITLE_BAR);
    }

    #[test]
    fn a_phone_reserves_a_title_bar_and_a_control_bar() {
        let expected = TITLE_BAR + CONTROL_BAR;
        assert_eq!(vertical_inset(Some(MirrorKind::Mirror)), expected);
        assert_eq!(vertical_inset(Some(MirrorKind::Camera)), expected);
        assert_eq!(vertical_inset(Some(MirrorKind::ScreenApp)), expected);
        assert_eq!(vertical_inset(None), expected);
    }

    #[test]
    fn an_app_reserves_a_title_bar_and_a_small_gap() {
        assert_eq!(
            vertical_inset(Some(MirrorKind::App)),
            TITLE_BAR + BOTTOM_GAP
        );
    }

    #[test]
    fn caps_the_long_side() {
        let (w, h) = fit_size(2400, 1080, 10_000.0);
        assert_eq!(w, MAX_LONG_SIDE);
        assert_eq!(h, MAX_LONG_SIDE * 1080.0 / 2400.0);
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

    #[test]
    fn display_pixels_drop_the_inset_then_apply_the_display_density() {
        // A 410x820 logical window at 2x on a 420 dpi phone: 410x778 dp in 1076x2042 pixels.
        assert_eq!(
            display_pixels((820, 1640), 2.0, 42.0, 420.0 / 160.0),
            Some((1076, 2042))
        );
        assert_eq!(
            display_pixels((100, 100), f64::NAN, 10.0, 1.0),
            Some((100, 90))
        );
    }

    #[test]
    fn a_minimized_window_has_no_display_pixels() {
        assert_eq!(display_pixels((0, 0), 1.0, 42.0, 1.0), None);
        assert_eq!(display_pixels((300, 40), 1.0, 42.0, 1.0), None);
    }
}
