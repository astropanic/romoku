use crate::board::Pos;

/// The camera is the world position shown at the centre of the viewport.
/// Storing the centre (not the top-left) keeps the view stable on resize.
#[derive(Default)]
pub struct Camera {
    pub center: Pos,
}

/// Visible rectangle of the world, in cells.
#[derive(Clone, Copy, Debug)]
pub struct View {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl Camera {
    pub fn view(&self, width: i32, height: i32) -> View {
        View {
            left: self.center.0 - width / 2,
            top: self.center.1 - height / 2,
            width,
            height,
        }
    }

    /// Scrolls just enough to keep `target` at least `margin` cells away
    /// from every edge (like vim's `scrolloff`). The margin shrinks on tiny
    /// viewports so it can always be satisfied.
    pub fn follow(&mut self, target: Pos, width: i32, height: i32, margin: i32) {
        self.center.0 += Self::shift(target.0, self.view(width, height).left, width, margin);
        self.center.1 += Self::shift(target.1, self.view(width, height).top, height, margin);
    }

    fn shift(target: i32, start: i32, size: i32, margin: i32) -> i32 {
        let margin = margin.min((size - 1) / 2).max(0);
        let lo = start + margin;
        let hi = start + size - 1 - margin;
        if target < lo {
            target - lo
        } else if target > hi {
            target - hi
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_move_while_target_inside_margin() {
        let mut cam = Camera::default();
        // view is x in -10..=10, y in -5..=5; with margin 3 the safe zone is x -7..=7, y -2..=2
        cam.follow((7, -2), 21, 11, 3);
        assert_eq!(cam.center, (0, 0));
    }

    #[test]
    fn scrolls_minimally_when_target_enters_margin() {
        let mut cam = Camera::default();
        // view is x in -10..=10, margin 3 => allowed up to 7
        cam.follow((8, 0), 21, 11, 3);
        assert_eq!(cam.center, (1, 0));
        cam.follow((-20, 0), 21, 11, 3);
        assert_eq!(cam.view(21, 11).left + 3, -20);
    }

    #[test]
    fn tiny_viewport_still_contains_target() {
        let mut cam = Camera::default();
        cam.follow((50, 50), 2, 1, 5);
        let v = cam.view(2, 1);
        assert!((v.left..v.left + v.width).contains(&50));
        assert!((v.top..v.top + v.height).contains(&50));
    }
}
