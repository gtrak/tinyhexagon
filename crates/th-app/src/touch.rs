//! plan 002 issue 07 — the kid's touch input: the two hold halves
//! (left/right rotation) + the two corner icons' hit tests (the adult
//! `th-additions` overlay is gone — this is the plan's small `touch.rs`).
//!
//! The window layer feeds each frame's down fingers + the window width;
//! [`TouchState`] binds every NEW finger to its down-half (a finger stays
//! bound to the half it came down on for the whole touch, so multi-touch
//! can hold both sides) and consumes a finger that comes down on an icon
//! (the icon fires a tap on the down-edge; the finger does NOT rotate).
//! The icon geometry is shared with the icon DRAW (`tiny.rs` draws the
//! two ~64×64 corner squares — rectangles/triangles, no fonts).

use th_render::input::FINGER_COUNT;
use th_render::Vec2;

/// One hold side (the engine's `inputImplCCW` / `inputImplCW`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The left half → counter-clockwise.
    Left,
    /// The right half → clockwise.
    Right,
}

/// The two corner icons (the hit-test / tap targets).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    /// Top-left: reset the current stage.
    Reset,
    /// Top-right: toggle fullscreen.
    Fullscreen,
}

/// The icon's hit square (~64×64, the plan's size).
pub const ICON_SIZE: f32 = 64.0;
/// The icon's margin from the window's top corner.
pub const ICON_MARGIN: f32 = 8.0;

/// The icon at the screen point `(x, y)` for a window `width` wide
/// (`None` = no icon — the point belongs to a rotation half).
#[must_use]
pub fn icon_at(x: f32, y: f32, width: f32) -> Option<Icon> {
    if (ICON_MARGIN..ICON_MARGIN + ICON_SIZE).contains(&x)
        && (ICON_MARGIN..ICON_MARGIN + ICON_SIZE).contains(&y)
    {
        return Some(Icon::Reset);
    }
    let fs_left = width - ICON_MARGIN - ICON_SIZE;
    if (fs_left..fs_left + ICON_SIZE).contains(&x)
        && (ICON_MARGIN..ICON_MARGIN + ICON_SIZE).contains(&y)
    {
        return Some(Icon::Fullscreen);
    }
    None
}

/// The icon's top-left screen position (the draw's origin;
/// [`ICON_SIZE`] × [`ICON_SIZE`]).
#[must_use]
pub fn icon_position(icon: Icon, width: f32) -> Vec2 {
    match icon {
        Icon::Reset => Vec2::new(ICON_MARGIN, ICON_MARGIN),
        Icon::Fullscreen => Vec2::new(width - ICON_MARGIN - ICON_SIZE, ICON_MARGIN),
    }
}

/// The per-frame touch tracking (the plan's two-zone touch):
/// multi-touch holds both sides (finger-captured halves); a finger on an
/// icon is consumed by the icon (no rotation, one tap on the down-edge).
#[derive(Debug, Clone, Default)]
pub struct TouchState {
    /// The finger's capture (`Free` = up — the capture dies with the
    /// touch; an icon-consumed finger stays consumed for the whole
    /// touch, so a held icon finger keeps no tap and no rotation).
    bound: [Bound; FINGER_COUNT],
}

/// One finger's capture for the duration of its touch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum Bound {
    /// The finger is up (or has not come down yet).
    #[default]
    Free,
    /// Captured the rotation half the finger came down on.
    Half(Side),
    /// Consumed by the corner icon the finger came down on.
    Icon(Icon),
}

impl TouchState {
    /// No fingers bound.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bound: [Bound::default(); FINGER_COUNT],
        }
    }

    /// One frame: `down` = the down fingers `(id, position)` (the window's
    /// `finger_down_positions` order — finger 0 first), `width` = the
    /// window width (the half-split boundary). Returns
    /// `(left_held, right_held, icon_tap)` — the held sides for the engine
    /// impl flags + the icon tap fired this frame (a finger's down-edge on
    /// an icon; held icon fingers keep no tap).
    #[must_use]
    pub fn feed(&mut self, down: &[(u32, Vec2)], width: f32) -> (bool, bool, Option<Icon>) {
        let mut left = false;
        let mut right = false;
        let mut tap: Option<Icon> = None;

        // Release the fingers that are up (their ids are absent from
        // `down`): the half capture dies with the touch.
        let down_mask = {
            let mut mask = [false; FINGER_COUNT];
            for (id, _) in down {
                if let Some(slot) = mask.get_mut(*id as usize) {
                    *slot = true;
                }
            }
            mask
        };
        for (id, slot) in self.bound.iter_mut().enumerate() {
            if !down_mask.get(id).copied().unwrap_or(false) {
                *slot = Bound::Free;
            }
        }

        for (id, pos) in down {
            if *id >= FINGER_COUNT as u32 {
                continue; // a finger id beyond the window's fingerCount
            }
            let idx = *id as usize;
            // The down-edge: an icon finger is CONSUMED by the icon (the
            // tap fires once, here); otherwise capture the half the
            // finger came down on. A held finger keeps its capture (the
            // plan's "track finger ids") — neither re-binds nor
            // re-fires.
            if matches!(self.bound[idx], Bound::Free) {
                if let Some(icon) = icon_at(pos.x, pos.y, width) {
                    self.bound[idx] = Bound::Icon(icon);
                    tap = Some(icon);
                } else if pos.x < width / 2.0 {
                    self.bound[idx] = Bound::Half(Side::Left);
                } else {
                    self.bound[idx] = Bound::Half(Side::Right);
                }
            }
            match self.bound[idx] {
                Bound::Half(Side::Left) => left = true,
                Bound::Half(Side::Right) => right = true,
                Bound::Free | Bound::Icon(_) => {}
            }
        }

        (left, right, tap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y)
    }

    /// The icon hit tests: the top-left / top-right ~64×64 squares, and
    /// nothing else (the rest of the window is rotation halves).
    #[test]
    fn the_icons_hit_test_their_corner_squares() {
        // The reset icon (top-left, 8..72 on both axes).
        assert_eq!(icon_at(8.0, 8.0, 1024.0), Some(Icon::Reset));
        assert_eq!(icon_at(32.0, 40.0, 1024.0), Some(Icon::Reset));
        assert_eq!(icon_at(71.0, 71.0, 1024.0), Some(Icon::Reset));
        // The fullscreen icon (top-right: 1024 - 8 - 64 = 952..1016).
        assert_eq!(icon_at(952.0, 8.0, 1024.0), Some(Icon::Fullscreen));
        assert_eq!(icon_at(1015.0, 71.0, 1024.0), Some(Icon::Fullscreen));
        // The margin / the rest of the window: no icon.
        assert_eq!(icon_at(0.0, 0.0, 1024.0), None);
        assert_eq!(icon_at(7.9, 40.0, 1024.0), None);
        assert_eq!(icon_at(40.0, 72.5, 1024.0), None);
        assert_eq!(icon_at(512.0, 40.0, 1024.0), None);
        assert_eq!(icon_at(951.9, 40.0, 1024.0), None);
    }

    /// A finger binds to its down-half and keeps it for the whole touch
    /// (a slide across the split does not re-bind — the capture); a new
    /// finger on the other side holds BOTH (multi-touch).
    #[test]
    fn fingers_bind_their_down_half_and_multi_touch_holds_both_sides() {
        let mut t = TouchState::new();
        // Finger 0 comes down on the left half → left held.
        let (l, r, tap) = t.feed(&[(0, at(100.0, 400.0))], 1024.0);
        assert!(l && !r && tap.is_none());
        // Finger 1 comes down on the right half → both held.
        let (l, r, tap) = t.feed(&[(0, at(100.0, 400.0)), (1, at(900.0, 400.0))], 1024.0);
        assert!(l && r && tap.is_none());
        // Finger 0 slides onto the right half: its bound LEFT half is
        // captured — still both (the finger id keeps its side).
        let (l, r, _) = t.feed(&[(0, at(900.0, 400.0)), (1, at(900.0, 400.0))], 1024.0);
        assert!(l && r);
        // Finger 1 lifts → the right side is released, the left holds.
        let (l, r, _) = t.feed(&[(0, at(900.0, 400.0))], 1024.0);
        assert!(l && !r);
        // Both lift → nothing held.
        let (l, r, _) = t.feed(&[], 1024.0);
        assert!(!l && !r);
    }

    /// A finger that comes down on an icon is consumed by the icon: the
    /// tap fires on the down-edge, the finger does NOT rotate (and a held
    /// icon finger keeps no tap).
    #[test]
    fn a_finger_on_an_icon_is_consumed_by_the_icon() {
        let mut t = TouchState::new();
        // Down on the reset icon: a tap, no rotation — even though the
        // icon sits in the left half.
        let (l, r, tap) = t.feed(&[(0, at(32.0, 32.0))], 1024.0);
        assert!(!l && !r && tap == Some(Icon::Reset));
        // Held on the icon: no tap, no rotation.
        let (l, r, tap) = t.feed(&[(0, at(33.0, 33.0))], 1024.0);
        assert!(!l && !r && tap.is_none());
        // Up on the icon: nothing.
        let (l, r, tap) = t.feed(&[], 1024.0);
        assert!(!l && !r && tap.is_none());
        // A second finger on the fullscreen icon while the first holds the
        // right half: both actions coexist (the tap + the held side).
        let (l, r, tap) = t.feed(&[(1, at(900.0, 400.0)), (2, at(984.0, 32.0))], 1024.0);
        assert!(!l && r && tap == Some(Icon::Fullscreen));
    }
}
