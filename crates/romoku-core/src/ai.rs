//! A simple one-ply CPU opponent.
//!
//! Every empty cell near existing stones is scored by looking at all the
//! 5-cell windows passing through it (5 offsets x 4 directions). A window
//! that contains only our stones is a chance to attack, one with only the
//! opponent's stones is a threat to block. Counting windows rather than
//! consecutive runs naturally handles gapped shapes like `XX_XX` and
//! rewards open lines (more live windows) over blocked ones.

use std::collections::HashSet;

use crate::board::{Board, DIRECTIONS, Player, Pos, WIN_LENGTH};

/// Score for completing a window that already has N of our stones.
/// Index 4 = this move makes five, which must beat everything.
const ATTACK: [i64; WIN_LENGTH] = [1, 10, 100, 10_000, 10_000_000];
/// Score for blocking a window that already has N opponent stones.
/// Index 4 = the opponent would make five next move, so we must block.
const DEFENSE: [i64; WIN_LENGTH] = [0, 5, 50, 5_000, 500_000];

/// Candidate cells are empty cells within this distance of any stone.
const NEIGHBORHOOD: i32 = 2;

pub fn best_move(board: &Board, me: Player) -> Pos {
    if board.is_empty() {
        return (0, 0);
    }
    let mut candidates: Vec<Pos> = candidates(board).into_iter().collect();
    // HashSet order is random; sort so the choice is deterministic.
    candidates.sort_unstable();
    candidates
        .into_iter()
        .max_by_key(|&pos| (score(board, pos, me), std::cmp::Reverse(pos)))
        .expect("a non-empty board always has empty neighbours")
}

fn candidates(board: &Board) -> HashSet<Pos> {
    let mut set = HashSet::new();
    for ((x, y), _) in board.stones() {
        for dy in -NEIGHBORHOOD..=NEIGHBORHOOD {
            for dx in -NEIGHBORHOOD..=NEIGHBORHOOD {
                let pos = (x + dx, y + dy);
                if board.get(pos).is_none() {
                    set.insert(pos);
                }
            }
        }
    }
    set
}

fn score(board: &Board, pos: Pos, me: Player) -> i64 {
    let mut total = 0;
    let len = WIN_LENGTH as i32;
    for (dx, dy) in DIRECTIONS {
        // Each window of WIN_LENGTH cells containing `pos`.
        for start in -(len - 1)..=0 {
            let (mut mine, mut theirs) = (0, 0);
            for i in start..start + len {
                match board.get((pos.0 + dx * i, pos.1 + dy * i)) {
                    Some(p) if p == me => mine += 1,
                    Some(_) => theirs += 1,
                    None => {}
                }
            }
            if theirs == 0 {
                total += ATTACK[mine];
            }
            if mine == 0 {
                total += DEFENSE[theirs];
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    fn board(stones: &[(Pos, Player)]) -> Board {
        let mut b = Board::default();
        for &(pos, p) in stones {
            assert!(b.place(pos, p));
        }
        b
    }

    #[test]
    fn opens_at_origin() {
        assert_eq!(best_move(&Board::default(), Player::X), (0, 0));
    }

    #[test]
    fn completes_five() {
        let b = board(&[
            ((0, 0), Player::O),
            ((1, 0), Player::O),
            ((2, 0), Player::O),
            ((3, 0), Player::O),
            ((4, 0), Player::X),
        ]);
        assert_eq!(best_move(&b, Player::O), (-1, 0));
    }

    #[test]
    fn completes_gapped_five() {
        let b = board(&[
            ((0, 0), Player::O),
            ((1, 1), Player::O),
            ((3, 3), Player::O),
            ((4, 4), Player::O),
        ]);
        assert_eq!(best_move(&b, Player::O), (2, 2));
    }

    #[test]
    fn prefers_winning_over_blocking() {
        let b = board(&[
            ((0, 0), Player::X),
            ((0, 1), Player::X),
            ((0, 2), Player::X),
            ((0, 3), Player::X),
            ((5, 0), Player::O),
            ((5, 1), Player::O),
            ((5, 2), Player::O),
            ((5, 3), Player::O),
        ]);
        let m = best_move(&b, Player::O);
        assert!(m == (5, 4) || m == (5, -1), "got {m:?}");
    }

    #[test]
    fn blocks_four() {
        let b = board(&[
            ((0, 0), Player::X),
            ((1, 0), Player::X),
            ((2, 0), Player::X),
            ((3, 0), Player::X),
            ((-1, 0), Player::O),
        ]);
        assert_eq!(best_move(&b, Player::O), (4, 0));
    }

    #[test]
    fn blocks_open_three() {
        let b = board(&[
            ((0, 0), Player::X),
            ((1, 1), Player::X),
            ((2, 2), Player::X),
            ((0, 5), Player::O),
        ]);
        let m = best_move(&b, Player::O);
        assert!(m == (-1, -1) || m == (3, 3), "got {m:?}");
    }
}
