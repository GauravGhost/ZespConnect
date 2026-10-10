//! Client-area sizing for in-app mirror windows: Tauri/tao has no aspect-ratio lock, so a
//! mirror window keeps the video's shape by hand, correcting the size with [`lock_size`] after
//! each resize.

use crate::models::MirrorKind;

/// Smallest client-area side a mirror window may shrink to; set as the OS minimum in both dimensions.
pub const MIN_SIDE: f64 = 240.0;

/// The longer side a window takes the first time it fits a session.
pub const DEFAULT_LONG_SIDE: f64 = 820.0;

/// Margin below the video so it does not run into the window's bottom edge; `MirrorViewer` pads its canvas by the same amount.
pub const BOTTOM_INSET: f64 = 14.0;

/// Height of a mirror window's own title bar, which replaces the native one on every platform.
pub const TITLE_BAR: f64 = 28.0;

/// The vertical space a window of `kind` reserves: a desktop's title bar only, or a phone's title bar plus `BOTTOM_INSET`.
pub fn vertical_inset(kind: Option<MirrorKind>) -> f64 {
    if kind == Some(MirrorKind::Desktop) {
        TITLE_BAR
    } else {
        TITLE_BAR + BOTTOM_INSET
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
    fn a_phone_reserves_a_title_bar_and_a_bottom_gap() {
        let expected = TITLE_BAR + BOTTOM_INSET;
        assert_eq!(vertical_inset(Some(MirrorKind::App)), expected);
        assert_eq!(vertical_inset(Some(MirrorKind::ScreenApp)), expected);
        assert_eq!(vertical_inset(None), expected);
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
}
