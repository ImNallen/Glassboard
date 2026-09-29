#[derive(Clone, Copy)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// All coordinates are physical pixels, including on mixed-DPI displays.
pub fn anchored_position(icon: Bounds, work: Bounds, size: (f64, f64), scale: f64) -> (i32, i32) {
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
    (x.round() as i32, y.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opens_below_top_bar_and_above_bottom_taskbar() {
        let work = Bounds {
            x: 0.0,
            y: 24.0,
            width: 1440.0,
            height: 836.0,
        };
        let top = Bounds {
            x: 700.0,
            y: 0.0,
            width: 24.0,
            height: 24.0,
        };
        let bottom = Bounds { y: 860.0, ..top };
        assert_eq!(anchored_position(top, work, (360.0, 316.0), 1.0), (532, 28));
        assert_eq!(
            anchored_position(bottom, work, (360.0, 316.0), 1.0),
            (532, 540)
        );
    }
    #[test]
    fn clamps_at_edges_and_handles_negative_monitor_origins_and_scaling() {
        let work = Bounds {
            x: -2880.0,
            y: 48.0,
            width: 2880.0,
            height: 1672.0,
        };
        let left = Bounds {
            x: -2880.0,
            y: 0.0,
            width: 48.0,
            height: 48.0,
        };
        let right = Bounds { x: -48.0, ..left };
        assert_eq!(
            anchored_position(left, work, (720.0, 632.0), 2.0),
            (-2872, 56)
        );
        assert_eq!(
            anchored_position(right, work, (720.0, 632.0), 2.0),
            (-728, 56)
        );
    }
    #[test]
    fn keeps_side_taskbar_and_small_screen_positions_in_the_work_area() {
        let work = Bounds {
            x: 40.0,
            y: 0.0,
            width: 600.0,
            height: 400.0,
        };
        let icon = Bounds {
            x: 0.0,
            y: 350.0,
            width: 40.0,
            height: 30.0,
        };
        assert_eq!(anchored_position(icon, work, (360.0, 316.0), 1.0), (44, 32));
        let tiny = Bounds {
            width: 200.0,
            height: 200.0,
            ..work
        };
        assert_eq!(anchored_position(icon, tiny, (360.0, 316.0), 1.0), (44, 4));
    }
}
