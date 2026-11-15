use std::collections::HashMap;

/// World coordinate on the infinite board. `y` grows downward (screen-like).
pub type Pos = (i32, i32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    X,
    O,
}

impl Player {
    pub fn other(self) -> Self {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }
}

/// The four line directions; their negations cover the other four.
pub const DIRECTIONS: [Pos; 4] = [(1, 0), (0, 1), (1, 1), (1, -1)];

pub const WIN_LENGTH: usize = 5;

/// Sparse, unbounded board: only occupied cells are stored.
#[derive(Clone, Default)]
pub struct Board {
    cells: HashMap<Pos, Player>,
}

impl Board {
    pub fn get(&self, pos: Pos) -> Option<Player> {
        self.cells.get(&pos).copied()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn stones(&self) -> impl Iterator<Item = (Pos, Player)> + '_ {
        self.cells.iter().map(|(&pos, &p)| (pos, p))
    }

    /// Places a stone. Returns false if the cell is already occupied.
    pub fn place(&mut self, pos: Pos, player: Player) -> bool {
        if self.cells.contains_key(&pos) {
            return false;
        }
        self.cells.insert(pos, player);
        true
    }

    /// If the stone at `pos` completes a line of WIN_LENGTH or more,
    /// returns every position in that line.
    pub fn winning_line(&self, pos: Pos) -> Option<Vec<Pos>> {
        let player = self.get(pos)?;
        for (dx, dy) in DIRECTIONS {
            let mut line = vec![pos];
            for sign in [1, -1] {
                let (mut x, mut y) = pos;
                loop {
                    x += dx * sign;
                    y += dy * sign;
                    if self.get((x, y)) != Some(player) {
                        break;
                    }
                    line.push((x, y));
                }
            }
            if line.len() >= WIN_LENGTH {
                return Some(line);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_win_in_all_directions_including_negative_coords() {
        for (dx, dy) in DIRECTIONS {
            let mut b = Board::default();
            for i in -2..=2 {
                b.place((-100 + i * dx, 50 + i * dy), Player::O);
            }
            let line = b.winning_line((-100, 50)).expect("should win");
            assert_eq!(line.len(), 5);
        }
    }

    #[test]
    fn four_is_not_a_win_and_opponent_breaks_line() {
        let mut b = Board::default();
        for x in 0..4 {
            b.place((x, 0), Player::X);
        }
        b.place((4, 0), Player::O);
        assert!(b.winning_line((3, 0)).is_none());
    }

    #[test]
    fn cannot_place_on_occupied_cell() {
        let mut b = Board::default();
        assert!(b.place((0, 0), Player::X));
        assert!(!b.place((0, 0), Player::O));
        assert_eq!(b.get((0, 0)), Some(Player::X));
    }
}
