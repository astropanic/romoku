//! Game flow: turns, modes, and moves from non-human controllers.
//!
//! This module does no I/O and starts no threads, so it runs anywhere
//! (terminal, browser/WASM, server). When a non-human side must move, the
//! frontend asks for a [`MoveRequest`], computes or receives the move however
//! it likes (thread, timer, network), and hands it back with
//! [`App::submit_move`].

use crate::board::{Board, Player, Pos};

/// Who makes the moves for a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Controller {
    Human,
    Cpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    HumanVsCpu,
    CpuVsHuman,
    HumanVsHuman,
    CpuVsCpu,
}

impl Mode {
    pub const ALL: [Mode; 4] = [
        Mode::HumanVsCpu,
        Mode::CpuVsHuman,
        Mode::HumanVsHuman,
        Mode::CpuVsCpu,
    ];

    pub fn controller(self, player: Player) -> Controller {
        use Controller::*;
        let (x, o) = match self {
            Mode::HumanVsCpu => (Human, Cpu),
            Mode::CpuVsHuman => (Cpu, Human),
            Mode::HumanVsHuman => (Human, Human),
            Mode::CpuVsCpu => (Cpu, Cpu),
        };
        match player {
            Player::X => x,
            Player::O => o,
        }
    }

    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&m| m == self).unwrap();
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::HumanVsCpu => "You (X) vs CPU (O)",
            Mode::CpuVsHuman => "CPU (X) vs You (O)",
            Mode::HumanVsHuman => "Human vs Human",
            Mode::CpuVsCpu => "CPU vs CPU",
        }
    }
}

/// "Please produce a move for `player` on `board`." The `ticket` identifies
/// the request so late answers (after a restart) are ignored.
#[derive(Clone)]
pub struct MoveRequest {
    pub ticket: u64,
    pub board: Board,
    pub player: Player,
}

pub struct App {
    pub mode: Mode,
    pub board: Board,
    pub cursor: Pos,
    pub turn: Player,
    pub last_move: Option<Pos>,
    pub winner: Option<(Player, Vec<Pos>)>,
    pub moves: usize,
    /// Ticket of the request currently being worked on, if any.
    outstanding: Option<u64>,
    /// Survives restarts so tickets are never reused.
    next_ticket: u64,
}

impl App {
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            board: Board::default(),
            cursor: (0, 0),
            turn: Player::X,
            last_move: None,
            winner: None,
            moves: 0,
            outstanding: None,
            next_ticket: 0,
        }
    }

    pub fn current_controller(&self) -> Controller {
        self.mode.controller(self.turn)
    }

    /// True while a non-human move has been requested but not delivered.
    pub fn is_waiting(&self) -> bool {
        self.outstanding.is_some()
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        self.cursor.0 += dx;
        self.cursor.1 += dy;
    }

    /// Human input: place at `pos`, but only on a human's turn.
    pub fn place_human(&mut self, pos: Pos) -> bool {
        self.current_controller() == Controller::Human && self.play(pos)
    }

    pub fn place_at_cursor(&mut self) -> bool {
        self.place_human(self.cursor)
    }

    /// Returns a request if a non-human side must move and nobody is working
    /// on it yet. Call this every frame/tick; it returns `Some` once per move.
    pub fn take_move_request(&mut self) -> Option<MoveRequest> {
        if self.winner.is_some()
            || self.outstanding.is_some()
            || self.current_controller() == Controller::Human
        {
            return None;
        }
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        self.outstanding = Some(ticket);
        Some(MoveRequest {
            ticket,
            board: self.board.clone(),
            player: self.turn,
        })
    }

    /// Delivers the answer to a request. Stale or unknown tickets are ignored.
    pub fn submit_move(&mut self, ticket: u64, pos: Pos) -> bool {
        if self.outstanding != Some(ticket) {
            return false;
        }
        self.outstanding = None;
        self.cursor = pos; // jump to the reply so the view shows it
        self.play(pos)
    }

    fn play(&mut self, pos: Pos) -> bool {
        if self.winner.is_some() || !self.board.place(pos, self.turn) {
            return false;
        }
        self.moves += 1;
        self.last_move = Some(pos);
        if let Some(line) = self.board.winning_line(pos) {
            self.winner = Some((self.turn, line));
        } else {
            self.turn = self.turn.other();
        }
        true
    }

    pub fn restart(&mut self) {
        self.set_mode(self.mode);
    }

    pub fn set_mode(&mut self, mode: Mode) {
        let next_ticket = self.next_ticket;
        *self = Self::new(mode);
        self.next_ticket = next_ticket;
    }

    pub fn cycle_mode(&mut self) {
        self.set_mode(self.mode.next());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai;

    /// Answers every pending request synchronously, like a frontend would.
    fn serve_cpu(app: &mut App) {
        while let Some(req) = app.take_move_request() {
            let pos = ai::best_move(&req.board, req.player);
            assert!(app.submit_move(req.ticket, pos));
        }
    }

    #[test]
    fn cpu_replies_to_human_move() {
        let mut app = App::new(Mode::HumanVsCpu);
        assert!(app.take_move_request().is_none(), "human moves first");
        assert!(app.place_at_cursor());
        serve_cpu(&mut app);
        assert_eq!(app.moves, 2);
        assert_eq!(app.turn, Player::X);
        assert_eq!(app.cursor, app.last_move.unwrap());
    }

    #[test]
    fn request_is_handed_out_once() {
        let mut app = App::new(Mode::CpuVsHuman);
        assert!(app.take_move_request().is_some());
        assert!(app.take_move_request().is_none());
        assert!(app.is_waiting());
    }

    #[test]
    fn human_cannot_move_on_cpu_turn() {
        let mut app = App::new(Mode::CpuVsHuman);
        assert!(!app.place_at_cursor());
        assert_eq!(app.moves, 0);
    }

    #[test]
    fn stale_answer_after_restart_is_ignored() {
        let mut app = App::new(Mode::CpuVsHuman);
        let old = app.take_move_request().unwrap();
        app.restart();
        let new = app.take_move_request().unwrap();
        assert_ne!(old.ticket, new.ticket);
        assert!(!app.submit_move(old.ticket, (5, 5)));
        assert!(app.submit_move(new.ticket, (0, 0)));
        assert_eq!(app.moves, 1);
    }

    #[test]
    fn cpu_vs_cpu_game_ends_with_a_winner() {
        let mut app = App::new(Mode::CpuVsCpu);
        serve_cpu(&mut app);
        assert!(app.winner.is_some());
    }
}
