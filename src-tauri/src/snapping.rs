//! Pure physical-pixel geometry used by the Windows move loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

pub fn snap(rect: Rect, work: Rect, notes: &[Rect], distance: i32) -> Rect {
    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    let mut dx = distance + 1;
    let mut dy = distance + 1;
    let mut choose_x = |delta: i32| { if delta.abs() <= distance && delta.abs() < dx.abs() { dx = delta; } };
    let mut choose_y = |delta: i32| { if delta.abs() <= distance && delta.abs() < dy.abs() { dy = delta; } };
    choose_x(work.left - rect.left);
    choose_x(work.right - rect.right);
    choose_y(work.top - rect.top);
    choose_y(work.bottom - rect.bottom);
    for other in notes {
        if rect.top <= other.bottom + distance && rect.bottom >= other.top - distance {
            for delta in [other.left - rect.right, other.right - rect.left, other.left - rect.left, other.right - rect.right] { choose_x(delta); }
        }
        if rect.left <= other.right + distance && rect.right >= other.left - distance {
            for delta in [other.top - rect.bottom, other.bottom - rect.top, other.top - rect.top, other.bottom - rect.bottom] { choose_y(delta); }
        }
    }
    let left = rect.left + if dx.abs() <= distance { dx } else { 0 };
    let top = rect.top + if dy.abs() <= distance { dy } else { 0 };
    Rect { left, top, right: left + width, bottom: top + height }
}

#[cfg(test)]
mod tests {
    use super::*;
    const WORK: Rect = Rect { left: -1920, top: 0, right: 0, bottom: 1040 };
    #[test]
    fn snaps_to_work_area_on_negative_monitor_and_preserves_size() {
        let rect = Rect { left: -1911, top: 7, right: -1631, bottom: 267 };
        assert_eq!(snap(rect, WORK, &[], 14), Rect { left: -1920, top: 0, right: -1640, bottom: 260 });
        let rect = Rect { left: -290, top: 790, right: -10, bottom: 1050 };
        assert_eq!(snap(rect, WORK, &[], 14), Rect { left: -280, top: 780, right: 0, bottom: 1040 });
    }
    #[test]
    fn joins_notes_but_detaches_outside_threshold() {
        let other = Rect { left: -1200, top: 200, right: -920, bottom: 460 };
        let rect = Rect { left: -913, top: 206, right: -633, bottom: 466 };
        assert_eq!(snap(rect, WORK, &[other], 14), Rect { left: -920, top: 200, right: -640, bottom: 460 });
        let far = Rect { left: -890, top: 500, right: -610, bottom: 760 };
        assert_eq!(snap(far, WORK, &[other], 14), far);
    }

    #[test]
    fn all_four_corners_keep_note_dimensions() {
        for (left, top) in [(WORK.left + 5, 5), (-285, 5), (WORK.left + 5, 785), (-285, 785)] {
            let moved = snap(Rect { left, top, right: left + 280, bottom: top + 260 }, WORK, &[], 14);
            assert_eq!((moved.right - moved.left, moved.bottom - moved.top), (280, 260));
            assert!(moved.left == WORK.left || moved.right == WORK.right);
            assert!(moved.top == WORK.top || moved.bottom == WORK.bottom);
        }
    }
}
