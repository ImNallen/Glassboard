use crate::Result;
use serde::{Deserialize, Serialize};
use tauri::{PhysicalPosition, PhysicalRect, PhysicalSize};

/// Logical window sizes. The toolbar's includes 12 logical pixels around it for its shadow.
pub(crate) const TOOLBAR_SIZE: (f64, f64) = (744.0, 76.0);
pub(crate) const SETTINGS_SIZE: (f64, f64) = (380.0, 620.0);
pub(crate) const TUTORIAL_SIZE: (f64, f64) = (400.0, 330.0);

/// A rectangle in physical pixels, unless a method says otherwise.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct Rect {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

impl From<PhysicalRect<i32, u32>> for Rect {
    fn from(PhysicalRect { position, size }: PhysicalRect<i32, u32>) -> Self {
        Self {
            x: position.x.into(),
            y: position.y.into(),
            width: size.width.into(),
            height: size.height.into(),
        }
    }
}

impl Rect {
    pub(crate) fn work_area(monitor: &tauri::Monitor) -> Self {
        (*monitor.work_area()).into()
    }
    fn map(self, f: fn(f64) -> f64) -> Self {
        Self {
            x: f(self.x),
            y: f(self.y),
            width: f(self.width),
            height: f(self.height),
        }
    }
    /// Whether a point is inside this rectangle or within `margin` of it.
    pub(crate) fn contains(&self, x: f64, y: f64, margin: f64) -> bool {
        x >= self.x - margin
            && y >= self.y - margin
            && x < self.x + self.width + margin
            && y < self.y + self.height + margin
    }
    /// This rectangle in CSS pixels of a webview whose top-left is at `origin`.
    pub(crate) fn in_css(self, origin: PhysicalPosition<i32>, scale: f64) -> Self {
        Self {
            x: (self.x - f64::from(origin.x)) / scale,
            y: (self.y - f64::from(origin.y)) / scale,
            width: self.width / scale,
            height: self.height / scale,
        }
    }
    /// A logical `size` in physical pixels, no larger than this area less `inset`
    /// logical pixels and no smaller than `min` physical pixels.
    pub(crate) fn fit(&self, size: (f64, f64), scale: f64, inset: f64, min: f64) -> (f64, f64) {
        (
            (size.0 * scale).min((self.width - inset * scale).max(min)),
            (size.1 * scale).min((self.height - inset * scale).max(min)),
        )
    }
    pub(crate) fn place(self, window: &tauri::WebviewWindow) -> Result<()> {
        window.set_position(PhysicalPosition::new(self.x, self.y))?;
        Ok(super::set_size(
            window,
            PhysicalSize::new(self.width, self.height),
        )?)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolbarPosition {
    Left,
    Right,
    Bottom,
}

/// The toolbar window centered on one edge of the work area.
pub(crate) fn dock(position: ToolbarPosition, work: Rect, scale: f64, expanded: bool) -> Rect {
    let (long, short) = TOOLBAR_SIZE;
    let size = match position {
        ToolbarPosition::Bottom => (long, if expanded { 176.0 } else { short }),
        _ => (if expanded { 386.0 } else { short }, long),
    };
    let margin = 8.0 * scale;
    let (width, height) = work.fit(size, scale, 16.0, 1.0);
    let x = match position {
        ToolbarPosition::Left => work.x + margin,
        ToolbarPosition::Right => work.x + work.width - margin - width,
        ToolbarPosition::Bottom => work.x + (work.width - width) / 2.0,
    };
    let y = match position {
        ToolbarPosition::Bottom => work.y + work.height - margin - height,
        _ => work.y + (work.height - height) / 2.0,
    };
    Rect {
        x,
        y,
        width,
        height,
    }
    .map(f64::round)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct TrackingLayout {
    pub(crate) dock: Rect,
    pub(crate) window: Rect,
    pub(crate) scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub(crate) struct ToolbarPointer {
    pub(crate) near: bool,
    pub(crate) x: f64,
    pub(crate) y: f64,
}

impl TrackingLayout {
    pub(crate) fn pointer_at(&self, x: f64, y: f64, margin: f64) -> Option<ToolbarPointer> {
        let near = self.dock.contains(x, y, margin * self.scale);
        if !near && !self.window.contains(x, y, 0.0) {
            return None;
        }
        Some(ToolbarPointer {
            near,
            x: (x - self.window.x) / self.scale,
            y: (y - self.window.y) / self.scale,
        })
    }
}

/// A popover of physical `size` below or above the tray icon, whichever fits.
pub(crate) fn beside_icon(icon: Rect, work: Rect, size: (f64, f64), scale: f64) -> Rect {
    let margin = 4.0 * scale;
    let gap = 2.0 * scale;
    let min_x = work.x + margin;
    let max_x = (work.x + work.width - size.0 - margin).max(min_x);
    let min_y = work.y + margin;
    let max_y = (work.y + work.height - size.1 - margin).max(min_y);
    let below = icon.y + icon.height + gap;
    let above = icon.y - size.1 - gap;
    let prefer_below = icon.y + icon.height / 2.0 < work.y + work.height / 2.0;
    let (preferred, alternate) = if prefer_below {
        (below, above)
    } else {
        (above, below)
    };
    let y = if (min_y..=max_y).contains(&preferred) {
        preferred
    } else if (min_y..=max_y).contains(&alternate) {
        alternate
    } else {
        preferred.clamp(min_y, max_y)
    };
    let x = (icon.x + icon.width / 2.0 - size.0 / 2.0).clamp(min_x, max_x);
    Rect {
        x,
        y,
        width: size.0,
        height: size.1,
    }
    .map(f64::round)
}

/// A window of physical `size` centered 20 logical pixels below the top of the work area.
pub(crate) fn top_center(work: Rect, size: (f64, f64), scale: f64) -> Rect {
    Rect {
        x: work.x + (work.width - size.0) / 2.0,
        y: work.y + (20.0 * scale).min((work.height - size.1).max(0.0)),
        width: size.0,
        height: size.1,
    }
    .map(f64::floor)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn capture_dock_preserves_native_coordinates_with_scaling_and_negative_origins() {
        let work = rect(-2520.0, 70.0, 2460.0, 1370.0);
        let origin = PhysicalPosition::new(-2560, 0);
        let scale = 2.0;
        for position in [
            ToolbarPosition::Left,
            ToolbarPosition::Right,
            ToolbarPosition::Bottom,
        ] {
            let native = dock(position, work, scale, false);
            let css = native.in_css(origin, scale);
            assert_eq!(css.x * scale + f64::from(origin.x), native.x);
            assert_eq!(css.y * scale + f64::from(origin.y), native.y);
            assert_eq!(css.width * scale, native.width);
            assert_eq!(css.height * scale, native.height);
        }
    }
    #[test]
    fn cursor_samples_use_window_css_coordinates_in_every_dock_position() {
        let scale = 2.0;
        let work = rect(-2880.0, 48.0, 2880.0, 1672.0);
        for position in [
            ToolbarPosition::Bottom,
            ToolbarPosition::Left,
            ToolbarPosition::Right,
        ] {
            let docked = dock(position, work, scale, false);
            for expanded in [false, true] {
                let window = dock(position, work, scale, expanded);
                let tracking = TrackingLayout {
                    dock: docked,
                    window,
                    scale,
                };
                let x = docked.x + 30.0 * scale;
                let y = docked.y + 30.0 * scale;
                let pointer = tracking.pointer_at(x, y, 20.0).unwrap();
                assert!(pointer.near);
                assert_eq!(pointer.x, (x - window.x) / scale);
                assert_eq!(pointer.y, (y - window.y) / scale);
                assert!(tracking.pointer_at(1000.0, -1000.0, 20.0).is_none());
            }
            let window = dock(position, work, scale, true);
            let tracking = TrackingLayout {
                dock: docked,
                window,
                scale,
            };
            let (x, y) = match position {
                ToolbarPosition::Left => (window.x + window.width - 30.0, window.y + 30.0),
                _ => (window.x + 30.0, window.y + 30.0),
            };
            // The tooltip stays hoverable without expanding the dock proximity area.
            assert!(!tracking.pointer_at(x, y, 20.0).unwrap().near);
        }
    }
    #[test]
    fn centers_the_toolbar_on_each_selected_edge() {
        let work = rect(0.0, 24.0, 1440.0, 836.0);
        assert_eq!(
            dock(ToolbarPosition::Bottom, work, 1.0, false),
            rect(348.0, 776.0, 744.0, 76.0)
        );
        assert_eq!(
            dock(ToolbarPosition::Left, work, 1.0, false),
            rect(8.0, 70.0, 76.0, 744.0)
        );
        assert_eq!(
            dock(ToolbarPosition::Right, work, 1.0, false),
            rect(1356.0, 70.0, 76.0, 744.0)
        );
    }
    #[test]
    fn error_panels_expand_inward_without_moving_the_toolbar() {
        let work = rect(0.0, 24.0, 1440.0, 836.0);
        for position in [
            ToolbarPosition::Left,
            ToolbarPosition::Right,
            ToolbarPosition::Bottom,
        ] {
            let closed = dock(position, work, 1.0, false);
            let open = dock(position, work, 1.0, true);
            match position {
                ToolbarPosition::Left => assert_eq!((closed.x, closed.y), (open.x, open.y)),
                ToolbarPosition::Right => assert_eq!(
                    (closed.x + closed.width, closed.y),
                    (open.x + open.width, open.y)
                ),
                ToolbarPosition::Bottom => assert_eq!(
                    (closed.x, closed.y + closed.height),
                    (open.x, open.y + open.height)
                ),
            }
        }
    }
    #[test]
    fn handles_scaled_negative_coordinates_and_short_displays() {
        let work = rect(-2880.0, 48.0, 2880.0, 1672.0);
        assert_eq!(
            dock(ToolbarPosition::Bottom, work, 2.0, false),
            rect(-2184.0, 1552.0, 1488.0, 152.0)
        );
        let small = rect(0.0, 0.0, 500.0, 450.0);
        let side = dock(ToolbarPosition::Left, small, 1.0, false);
        assert_eq!((side.y, side.height), (8.0, 434.0));
        let bottom = dock(ToolbarPosition::Bottom, small, 1.0, false);
        assert_eq!((bottom.x, bottom.width), (8.0, 484.0));
    }
    #[test]
    fn proximity_includes_a_margin_around_the_window() {
        let layout = rect(100.0, 200.0, 638.0, 76.0);
        assert!(layout.contains(100.0, 200.0, 0.0));
        assert!(layout.contains(737.0, 275.0, 0.0));
        assert!(!layout.contains(738.0, 275.0, 0.0));
        assert!(!layout.contains(99.0, 200.0, 0.0));
        assert!(layout.contains(80.0, 180.0, 20.0));
        assert!(layout.contains(757.0, 295.0, 20.0));
        assert!(!layout.contains(758.0, 295.0, 20.0));
        assert!(!layout.contains(400.0, 150.0, 20.0));
    }

    fn origin(rect: Rect) -> (f64, f64) {
        (rect.x, rect.y)
    }
    #[test]
    fn opens_below_top_bar_and_above_bottom_taskbar() {
        let work = rect(0.0, 24.0, 1440.0, 836.0);
        let top = rect(700.0, 0.0, 24.0, 24.0);
        let bottom = Rect { y: 860.0, ..top };
        assert_eq!(
            origin(beside_icon(top, work, (360.0, 316.0), 1.0)),
            (532.0, 28.0)
        );
        assert_eq!(
            origin(beside_icon(bottom, work, (360.0, 316.0), 1.0)),
            (532.0, 540.0)
        );
    }
    #[test]
    fn clamps_at_edges_and_handles_negative_monitor_origins_and_scaling() {
        let work = rect(-2880.0, 48.0, 2880.0, 1672.0);
        let left = rect(-2880.0, 0.0, 48.0, 48.0);
        let right = Rect { x: -48.0, ..left };
        assert_eq!(
            origin(beside_icon(left, work, (720.0, 632.0), 2.0)),
            (-2872.0, 56.0)
        );
        assert_eq!(
            origin(beside_icon(right, work, (720.0, 632.0), 2.0)),
            (-728.0, 56.0)
        );
    }
    #[test]
    fn keeps_side_taskbar_and_small_screen_positions_in_the_work_area() {
        let work = rect(40.0, 0.0, 600.0, 400.0);
        let icon = rect(0.0, 350.0, 40.0, 30.0);
        assert_eq!(
            origin(beside_icon(icon, work, (360.0, 316.0), 1.0)),
            (44.0, 32.0)
        );
        let tiny = Rect {
            width: 200.0,
            height: 200.0,
            ..work
        };
        assert_eq!(
            origin(beside_icon(icon, tiny, (360.0, 316.0), 1.0)),
            (44.0, 4.0)
        );
    }

    fn tutorial(work: Rect, scale: f64) -> Rect {
        top_center(work, work.fit(TUTORIAL_SIZE, scale, 0.0, 0.0), scale)
    }
    #[test]
    fn tutorial_sits_centered_below_the_top_of_the_work_area() {
        let work = rect(0.0, 24.0, 1440.0, 836.0);
        assert_eq!(tutorial(work, 1.0), rect(520.0, 44.0, 400.0, 330.0));
        let retina = rect(-2880.0, 48.0, 2880.0, 1672.0);
        assert_eq!(tutorial(retina, 2.0), rect(-1840.0, 88.0, 800.0, 660.0));
    }
    #[test]
    fn tutorial_rounds_fractional_scales_down_on_negative_origins() {
        let work = rect(-1921.0, 0.0, 1921.0, 1080.0);
        assert_eq!(tutorial(work, 1.25), rect(-1211.0, 25.0, 500.0, 412.0));
    }
    #[test]
    fn tutorial_shrinks_to_a_short_display_and_drops_its_top_gap() {
        let work = rect(100.0, 0.0, 300.0, 330.0);
        assert_eq!(tutorial(work, 1.0), rect(100.0, 0.0, 300.0, 330.0));
        let roomy = Rect {
            height: 340.0,
            ..work
        };
        assert_eq!(tutorial(roomy, 1.0).y, 10.0);
    }
}
