//! Where the widget opens. Pure geometry in logical points, so every platform's layout
//! (menu bar, taskbar at any edge, GNOME's top panel, mixed scale factors) is unit-tested
//! without a real screen. `lib.rs` reads the monitors and moves the window.

/// A rectangle in logical points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Area {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Area { x, y, w, h }
    }
    fn right(&self) -> f64 {
        self.x + self.w
    }
    fn bottom(&self) -> f64 {
        self.y + self.h
    }
    fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
    fn contains(&self, (x, y): (f64, f64)) -> bool {
        (self.x..self.right()).contains(&x) && (self.y..self.bottom()).contains(&y)
    }
}

/// One monitor: its full bounds, the part not covered by menu bar / taskbar / panels
/// (the work area), and its scale factor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Screen {
    pub bounds: Area,
    pub work: Area,
    pub scale: f64,
}

/// Top-left corner of the widget, and whether it grows upwards (anchored at its bottom,
/// above a bottom taskbar) or downwards (hanging from a top bar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub grow_up: bool,
}

/// Distance from the bar or taskbar edge when anchored to the tray icon.
const NEAR_GAP: f64 = 6.0;
/// Distance from the screen edges when there is no icon position.
const CORNER_GAP: f64 = 8.0;

/// `v` kept within `[lo, hi]`; when the range is empty (a window larger than the work
/// area), `lo` wins, so the top-left stays visible. `f64::clamp` would panic instead.
fn fit(v: f64, lo: f64, hi: f64) -> f64 {
    v.min(hi).max(lo)
}

/// The screen the tray icon is on, and the icon in that screen's logical points.
///
/// `icon_at(scale)` gives the icon rect converted with a given scale factor: tray rects
/// come in physical pixels of the icon's *own* screen, so each screen is tried with its
/// own scale. A rect at the origin is the placeholder macOS reports right after launch
/// (or when the menu bar is full), and matches nothing.
///
/// On macOS each screen's physical space is its points times its own scale, so those
/// spaces overlap and a rect can fall inside two screens (a Retina laptop plus a smaller
/// 1x monitor to its right). Menu-bar icons are as tall as their screen's menu bar, so
/// the screen whose menu bar height matches the icon wins the tie.
pub fn icon_screen(icon_at: impl Fn(f64) -> Area, screens: &[Screen]) -> Option<(usize, Area)> {
    let origin = icon_at(1.0);
    if (origin.x, origin.y) == (0.0, 0.0) {
        return None;
    }
    let candidates: Vec<(usize, Area)> = screens
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            let icon = icon_at(s.scale);
            (icon.h > 0.0 && s.bounds.contains(icon.center())).then_some((i, icon))
        })
        .collect();
    let menu_bar_mismatch = |&(i, icon): &(usize, Area)| {
        let bar = screens[i].work.y - screens[i].bounds.y;
        (icon.h - bar).abs()
    };
    match candidates.len() {
        0 => None,
        1 => candidates.first().copied(),
        _ => candidates
            .into_iter()
            .min_by(|a, b| menu_bar_mismatch(a).total_cmp(&menu_bar_mismatch(b))),
    }
}

/// Next to the tray icon: centred under it below a top bar, centred above it over a
/// bottom taskbar. Always inside the work area, which also keeps it beside a vertical
/// taskbar instead of on top of it.
pub fn near_icon(icon: Area, screen: &Screen, (w, h): (f64, f64)) -> Placement {
    let (b, work) = (screen.bounds, screen.work);
    let (cx, _) = icon.center();
    let x_in = |x: f64| fit(x, work.x + NEAR_GAP, work.right() - w - NEAR_GAP);
    let y_in = |y: f64| fit(y, work.y + NEAR_GAP, work.bottom() - h - NEAR_GAP);

    // Anchor to the edge of the bar itself, not to the icon: the icon rect macOS reports
    // can sit lower than the icon really is (logged 30, 15 and -3 for the same icon), and
    // taskbar icons sit inset from the taskbar's edge.
    let grow_up = icon.y > b.y + b.h / 2.0;
    let y = if grow_up {
        icon.y.min(work.bottom()) - h - NEAR_GAP
    } else {
        work.y + NEAR_GAP
    };
    Placement {
        x: x_in(cx - w / 2.0),
        y: y_in(y),
        grow_up,
    }
}

/// No usable icon position (Linux never reports one; macOS right after launch).
/// Top-right under a top bar, bottom-right above a bottom taskbar.
///
/// `prefer_top`: the platform has its bar at the top even when the work area does not
/// say so (macOS; GNOME on Wayland reports the full screen as work area).
pub fn corner(screen: &Screen, (w, h): (f64, f64), prefer_top: bool) -> Placement {
    let (b, work) = (screen.bounds, screen.work);
    let bar_on_top = work.y > b.y;
    let bar_on_bottom = work.bottom() < b.bottom();
    let at_top = prefer_top || (bar_on_top && !bar_on_bottom);
    let x = fit(
        work.right() - w - CORNER_GAP,
        work.x + CORNER_GAP,
        f64::INFINITY,
    );
    let y = if at_top {
        work.y + CORNER_GAP
    } else {
        fit(
            work.bottom() - h - CORNER_GAP,
            work.y + CORNER_GAP,
            f64::INFINITY,
        )
    };
    Placement {
        x,
        y,
        grow_up: !at_top,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The widget: 290 points wide (lib.rs WIDTH), a typical list height.
    const WIN: (f64, f64) = (290.0, 300.0);

    /// A screen described the way Tauri reports it: physical origin and size, scale, and
    /// the bars around the work area in logical points (top, bottom, left, right).
    fn screen(px: f64, py: f64, pw: f64, ph: f64, scale: f64, bars: [f64; 4]) -> Screen {
        let bounds = Area::new(px / scale, py / scale, pw / scale, ph / scale);
        let [top, bottom, left, right] = bars;
        Screen {
            bounds,
            work: Area::new(
                bounds.x + left,
                bounds.y + top,
                bounds.w - left - right,
                bounds.h - top - bottom,
            ),
            scale,
        }
    }

    /// A tray icon rect in physical pixels, as `icon_at` sees it for a given scale.
    fn icon_px(x: f64, y: f64, w: f64, h: f64) -> impl Fn(f64) -> Area {
        move |s| Area::new(x / s, y / s, w / s, h / s)
    }

    /// Where it opens next to an icon given in physical pixels.
    fn open(icon: impl Fn(f64) -> Area, screens: &[Screen]) -> (usize, Placement) {
        let (i, icon) = icon_screen(icon, screens).expect("icon on a screen");
        (i, near_icon(icon, &screens[i], WIN))
    }

    fn inside_work(p: Placement, s: &Screen) {
        assert!(
            p.x >= s.work.x && p.x + WIN.0 <= s.work.x + s.work.w,
            "x {p:?}"
        );
        assert!(
            p.y >= s.work.y && p.y + WIN.1 <= s.work.y + s.work.h,
            "y {p:?}"
        );
    }

    // ---------- macOS ----------

    /// MacBook Air 13" (1470x956 points at 2x), 24-point menu bar, Dock at the bottom.
    fn macbook() -> Screen {
        screen(0.0, 0.0, 2940.0, 1912.0, 2.0, [24.0, 70.0, 0.0, 0.0])
    }

    #[test]
    fn macos_hangs_from_the_menu_bar_centred_under_the_icon() {
        let s = [macbook()];
        // Icon at x=1200..1230 points, in the menu bar.
        let (i, p) = open(icon_px(2400.0, 0.0, 60.0, 48.0), &s);
        assert_eq!(i, 0);
        assert!(!p.grow_up);
        assert_eq!(p.y, 24.0 + NEAR_GAP);
        assert_eq!(p.x, 1215.0 - 145.0);
        inside_work(p, &s[0]);
    }

    #[test]
    fn macos_icon_rect_reported_too_low_still_hangs_from_the_bar() {
        // macOS sometimes reports the icon a few points off (logged 30, 15, -3 for one icon).
        let s = [macbook()];
        for y_pt in [-3.0, 15.0, 30.0] {
            let (_, p) = open(icon_px(2400.0, y_pt * 2.0, 60.0, 48.0), &s);
            assert_eq!(p.y, 24.0 + NEAR_GAP, "icon reported at y={y_pt}");
        }
    }

    #[test]
    fn macos_icon_near_the_right_edge_keeps_the_widget_on_screen() {
        let s = [macbook()];
        let (_, p) = open(icon_px(2900.0, 0.0, 40.0, 48.0), &s);
        assert_eq!(p.x, 1470.0 - 290.0 - NEAR_GAP);
        inside_work(p, &s[0]);
    }

    #[test]
    fn macos_notched_macbook_has_a_taller_menu_bar() {
        // MacBook Pro 14": 1512x982 points, 37-point menu bar around the notch.
        let s = [screen(0.0, 0.0, 3024.0, 1964.0, 2.0, [37.0, 0.0, 0.0, 0.0])];
        let (_, p) = open(icon_px(2600.0, 0.0, 64.0, 74.0), &s);
        assert_eq!(p.y, 37.0 + NEAR_GAP);
    }

    #[test]
    fn macos_placeholder_rect_after_launch_matches_no_screen() {
        assert_eq!(icon_screen(icon_px(0.0, 0.0, 0.0, 0.0), &[macbook()]), None);
        assert_eq!(
            icon_screen(icon_px(0.0, 0.0, 30.0, 24.0), &[macbook()]),
            None
        );
    }

    #[test]
    fn macos_retina_plus_1x_monitor_picks_the_right_screen() {
        // Retina laptop at 0..1470 points, a 1920x1080 1x monitor to its right.
        let screens = [
            macbook(),
            screen(1470.0, 0.0, 1920.0, 1080.0, 1.0, [24.0, 0.0, 0.0, 0.0]),
        ];
        // Icon on the external screen at x=3200 points (physical = points at 1x).
        let (i, p) = open(icon_px(3200.0, 0.0, 30.0, 24.0), &screens);
        assert_eq!(i, 1);
        assert_eq!(p.y, 24.0 + NEAR_GAP);
        inside_work(p, &screens[1]);
        // Icon on the laptop at x=1200 points.
        let (i, _) = open(icon_px(2400.0, 0.0, 60.0, 48.0), &screens);
        assert_eq!(i, 0);
    }

    #[test]
    fn macos_overlapping_physical_spaces_break_the_tie_by_menu_bar_height() {
        // A narrower 1280x800 1x monitor right of the laptop (1470..2750 points). Its
        // physical space (1470..2750) overlaps the laptop's (0..2940): an icon at 2700 px
        // would read as 1350 points on the laptop. The menu bar height settles it.
        let screens = [
            macbook(),
            screen(1470.0, 0.0, 1280.0, 800.0, 1.0, [24.0, 0.0, 0.0, 0.0]),
        ];
        let (i, _) = open(icon_px(2700.0, 0.0, 30.0, 24.0), &screens);
        assert_eq!(i, 1, "a 24-px tall icon is a 1x menu bar icon");
        let (i, _) = open(icon_px(2700.0, 0.0, 60.0, 48.0), &screens);
        assert_eq!(i, 0, "a 48-px tall icon is a 2x menu bar icon");
    }

    #[test]
    fn macos_without_icon_position_uses_the_top_right_corner() {
        let p = corner(&macbook(), WIN, true);
        assert!(!p.grow_up);
        assert_eq!((p.x, p.y), (1470.0 - 290.0 - CORNER_GAP, 24.0 + CORNER_GAP));
    }

    // ---------- Windows ----------

    /// 1920x1080 at 100% with a 48-pixel taskbar at the bottom (the default).
    fn windows_100() -> Screen {
        screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [0.0, 48.0, 0.0, 0.0])
    }

    #[test]
    fn windows_bottom_taskbar_opens_above_the_icon() {
        let s = [windows_100()];
        let (_, p) = open(icon_px(1700.0, 1036.0, 40.0, 40.0), &s);
        assert!(p.grow_up);
        assert_eq!(
            p.y,
            1032.0 - 300.0 - NEAR_GAP,
            "just above the taskbar edge"
        );
        assert_eq!(p.x, 1720.0 - 145.0);
        inside_work(p, &s[0]);
    }

    #[test]
    fn windows_150_percent_scaling() {
        // 2560x1440 at 150%: 1706.7x960 points, 48-point taskbar.
        let s = [screen(0.0, 0.0, 2560.0, 1440.0, 1.5, [0.0, 48.0, 0.0, 0.0])];
        let (_, p) = open(icon_px(2300.0, 1380.0, 60.0, 60.0), &s);
        assert!(p.grow_up);
        assert!((p.y - (912.0 - 300.0 - NEAR_GAP)).abs() < 1e-9);
        inside_work(p, &s[0]);
    }

    #[test]
    fn windows_top_taskbar_opens_below_it() {
        let s = [screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [48.0, 0.0, 0.0, 0.0])];
        let (_, p) = open(icon_px(1700.0, 4.0, 40.0, 40.0), &s);
        assert!(!p.grow_up);
        assert_eq!(p.y, 48.0 + NEAR_GAP);
        inside_work(p, &s[0]);
    }

    #[test]
    fn windows_left_taskbar_opens_beside_it_not_on_top_of_it() {
        // 62-pixel vertical taskbar on the left, tray at its bottom.
        let s = [screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [0.0, 0.0, 62.0, 0.0])];
        let (_, p) = open(icon_px(11.0, 1000.0, 40.0, 40.0), &s);
        assert_eq!(p.x, 62.0 + NEAR_GAP, "right of the taskbar");
        assert!(p.grow_up);
        inside_work(p, &s[0]);
    }

    #[test]
    fn windows_right_taskbar_opens_beside_it() {
        let s = [screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [0.0, 0.0, 0.0, 62.0])];
        let (_, p) = open(icon_px(1869.0, 1000.0, 40.0, 40.0), &s);
        assert_eq!(p.x, 1858.0 - 290.0 - NEAR_GAP, "left of the taskbar");
        inside_work(p, &s[0]);
    }

    #[test]
    fn windows_second_monitor_left_of_the_primary() {
        // Primary at 0..1920, a second 1920x1080 monitor at -1920..0, taskbar on both.
        let screens = [
            windows_100(),
            screen(-1920.0, 0.0, 1920.0, 1080.0, 1.0, [0.0, 48.0, 0.0, 0.0]),
        ];
        let (i, p) = open(icon_px(-300.0, 1036.0, 40.0, 40.0), &screens);
        assert_eq!(i, 1);
        inside_work(p, &screens[1]);
    }

    #[test]
    fn windows_auto_hidden_taskbar_falls_back_to_the_screen_edge() {
        // Work area is the whole screen; the icon pops up at the bottom edge.
        let s = [screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [0.0, 0.0, 0.0, 0.0])];
        let (_, p) = open(icon_px(1700.0, 1040.0, 40.0, 40.0), &s);
        assert!(p.grow_up);
        inside_work(p, &s[0]);
    }

    #[test]
    fn windows_without_icon_position_uses_the_bottom_right_corner() {
        let p = corner(&windows_100(), WIN, false);
        assert!(p.grow_up);
        assert_eq!(
            (p.x, p.y),
            (1920.0 - 290.0 - CORNER_GAP, 1032.0 - 300.0 - CORNER_GAP)
        );
    }

    // ---------- Linux (no tray position: always the corner) ----------

    #[test]
    fn gnome_x11_top_panel_uses_the_top_right_corner() {
        // Ubuntu on X11: 32-point top bar, dock on the left.
        let s = screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [32.0, 0.0, 70.0, 0.0]);
        let p = corner(&s, WIN, false);
        assert!(!p.grow_up);
        assert_eq!(p.y, 32.0 + CORNER_GAP);
    }

    #[test]
    fn gnome_wayland_reports_no_panel_but_prefers_the_top() {
        // On Wayland the work area is the whole screen; the desktop hint decides.
        let s = screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [0.0, 0.0, 0.0, 0.0]);
        let p = corner(&s, WIN, true);
        assert!(!p.grow_up);
        assert_eq!(p.y, CORNER_GAP);
    }

    #[test]
    fn kde_bottom_panel_uses_the_bottom_right_corner() {
        let s = screen(0.0, 0.0, 1920.0, 1080.0, 1.0, [0.0, 44.0, 0.0, 0.0]);
        let p = corner(&s, WIN, false);
        assert!(p.grow_up);
        assert_eq!(p.y, 1036.0 - 300.0 - CORNER_GAP);
    }

    #[test]
    fn hidpi_linux_corner_stays_inside() {
        // 3840x2160 at 2x, GNOME top bar.
        let s = screen(0.0, 0.0, 3840.0, 2160.0, 2.0, [32.0, 0.0, 0.0, 0.0]);
        let p = corner(&s, WIN, false);
        assert_eq!((p.x, p.y), (1920.0 - 290.0 - CORNER_GAP, 32.0 + CORNER_GAP));
    }

    // ---------- Edge cases ----------

    #[test]
    fn a_window_taller_than_the_work_area_never_panics_and_keeps_its_top_visible() {
        let s = screen(0.0, 0.0, 800.0, 400.0, 1.0, [24.0, 0.0, 0.0, 0.0]);
        let tall = (290.0, 900.0);
        let p = near_icon(Area::new(700.0, 0.0, 20.0, 24.0), &s, tall);
        assert_eq!(p.y, 24.0 + NEAR_GAP);
        let p = corner(&s, tall, false);
        assert_eq!(p.y, 24.0 + CORNER_GAP);
        // And a window wider than the screen keeps its left edge visible.
        let p = near_icon(Area::new(700.0, 0.0, 20.0, 24.0), &s, (1000.0, 200.0));
        assert_eq!(p.x, NEAR_GAP);
    }

    #[test]
    fn an_icon_off_every_screen_matches_none() {
        assert_eq!(
            icon_screen(icon_px(5000.0, 5000.0, 30.0, 24.0), &[windows_100()]),
            None
        );
    }
}
