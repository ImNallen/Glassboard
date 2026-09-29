use crate::settings_position::Bounds;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolbarPosition {
    Left,
    Right,
    #[default]
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct TrackingLayout {
    pub dock: Layout,
    pub window: Layout,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct ToolbarPointer {
    pub near: bool,
    pub x: f64,
    pub y: f64,
}

impl TrackingLayout {
    pub fn pointer_at(&self, x: f64, y: f64, margin: f64) -> Option<ToolbarPointer> {
        let near = self.dock.is_near(x, y, margin * self.scale);
        if !near && !self.window.is_near(x, y, 0.0) {
            return None;
        }
        Some(ToolbarPointer {
            near,
            x: (x - self.window.x as f64) / self.scale,
            y: (y - self.window.y as f64) / self.scale,
        })
    }
}

impl Layout {
    /// Whether a physical cursor position is inside the toolbar window or within `margin` of it.
    pub fn is_near(&self, x: f64, y: f64, margin: f64) -> bool {
        x >= self.x as f64 - margin
            && y >= self.y as f64 - margin
            && x < self.x as f64 + self.width as f64 + margin
            && y < self.y as f64 + self.height as f64 + margin
    }
}

pub fn dock_layout(position: ToolbarPosition, work: Bounds, scale: f64, expanded: bool) -> Layout {
    // Include 12 logical pixels around the toolbar for its shadow.
    let (width, height) = match position {
        ToolbarPosition::Bottom => (744.0, if expanded { 176.0 } else { 76.0 }),
        _ => (if expanded { 386.0 } else { 76.0 }, 744.0),
    };
    let margin = 8.0 * scale;
    let width = (width * scale).min((work.width - 2.0 * margin).max(1.0));
    let height = (height * scale).min((work.height - 2.0 * margin).max(1.0));
    let x = match position {
        ToolbarPosition::Left => work.x + margin,
        ToolbarPosition::Right => work.x + work.width - margin - width,
        ToolbarPosition::Bottom => work.x + (work.width - width) / 2.0,
    };
    let y = match position {
        ToolbarPosition::Bottom => work.y + work.height - margin - height,
        _ => work.y + (work.height - height) / 2.0,
    };
    Layout {
        x: x.round() as i32,
        y: y.round() as i32,
        width: width.round() as u32,
        height: height.round() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursor_samples_use_window_css_coordinates_in_every_dock_position() {
        let scale = 2.0;
        let work = Bounds {
            x: -2880.0,
            y: 48.0,
            width: 2880.0,
            height: 1672.0,
        };
        for position in [
            ToolbarPosition::Bottom,
            ToolbarPosition::Left,
            ToolbarPosition::Right,
        ] {
            let dock = dock_layout(position, work, scale, false);
            for expanded in [false, true] {
                let window = dock_layout(position, work, scale, expanded);
                let tracking = TrackingLayout {
                    dock,
                    window,
                    scale,
                };
                let x = dock.x as f64 + 30.0 * scale;
                let y = dock.y as f64 + 30.0 * scale;
                let pointer = tracking.pointer_at(x, y, 20.0).unwrap();
                assert!(pointer.near);
                assert_eq!(pointer.x, (x - window.x as f64) / scale);
                assert_eq!(pointer.y, (y - window.y as f64) / scale);
                assert!(tracking.pointer_at(1000.0, -1000.0, 20.0).is_none());
            }
            let window = dock_layout(position, work, scale, true);
            let tracking = TrackingLayout {
                dock,
                window,
                scale,
            };
            let (x, y) = match position {
                ToolbarPosition::Left => (window.x + window.width as i32 - 30, window.y + 30),
                _ => (window.x + 30, window.y + 30),
            };
            // The tooltip stays hoverable without expanding the dock proximity area.
            assert!(!tracking.pointer_at(x as f64, y as f64, 20.0).unwrap().near);
        }
    }
    #[test]
    fn centers_the_toolbar_on_each_selected_edge() {
        let work = Bounds {
            x: 0.0,
            y: 24.0,
            width: 1440.0,
            height: 836.0,
        };
        assert_eq!(
            dock_layout(ToolbarPosition::Bottom, work, 1.0, false),
            Layout {
                x: 348,
                y: 776,
                width: 744,
                height: 76
            }
        );
        assert_eq!(
            dock_layout(ToolbarPosition::Left, work, 1.0, false),
            Layout {
                x: 8,
                y: 70,
                width: 76,
                height: 744
            }
        );
        assert_eq!(
            dock_layout(ToolbarPosition::Right, work, 1.0, false),
            Layout {
                x: 1356,
                y: 70,
                width: 76,
                height: 744
            }
        );
    }
    #[test]
    fn error_panels_expand_inward_without_moving_the_toolbar() {
        let work = Bounds {
            x: 0.0,
            y: 24.0,
            width: 1440.0,
            height: 836.0,
        };
        for position in [
            ToolbarPosition::Left,
            ToolbarPosition::Right,
            ToolbarPosition::Bottom,
        ] {
            let closed = dock_layout(position, work, 1.0, false);
            let open = dock_layout(position, work, 1.0, true);
            match position {
                ToolbarPosition::Left => assert_eq!((closed.x, closed.y), (open.x, open.y)),
                ToolbarPosition::Right => assert_eq!(
                    (closed.x + closed.width as i32, closed.y),
                    (open.x + open.width as i32, open.y)
                ),
                ToolbarPosition::Bottom => assert_eq!(
                    (closed.x, closed.y + closed.height as i32),
                    (open.x, open.y + open.height as i32)
                ),
            }
        }
    }
    #[test]
    fn handles_scaled_negative_coordinates_and_short_displays() {
        let work = Bounds {
            x: -2880.0,
            y: 48.0,
            width: 2880.0,
            height: 1672.0,
        };
        assert_eq!(
            dock_layout(ToolbarPosition::Bottom, work, 2.0, false),
            Layout {
                x: -2184,
                y: 1552,
                width: 1488,
                height: 152
            }
        );
        let small = Bounds {
            x: 0.0,
            y: 0.0,
            width: 500.0,
            height: 450.0,
        };
        let side = dock_layout(ToolbarPosition::Left, small, 1.0, false);
        assert_eq!((side.y, side.height), (8, 434));
        let bottom = dock_layout(ToolbarPosition::Bottom, small, 1.0, false);
        assert_eq!((bottom.x, bottom.width), (8, 484));
    }
    #[test]
    fn proximity_includes_a_margin_around_the_window() {
        let layout = Layout {
            x: 100,
            y: 200,
            width: 638,
            height: 76,
        };
        assert!(layout.is_near(100.0, 200.0, 0.0));
        assert!(layout.is_near(737.0, 275.0, 0.0));
        assert!(!layout.is_near(738.0, 275.0, 0.0));
        assert!(!layout.is_near(99.0, 200.0, 0.0));
        assert!(layout.is_near(80.0, 180.0, 20.0));
        assert!(layout.is_near(757.0, 295.0, 20.0));
        assert!(!layout.is_near(758.0, 295.0, 20.0));
        assert!(!layout.is_near(400.0, 150.0, 20.0));
    }
}
