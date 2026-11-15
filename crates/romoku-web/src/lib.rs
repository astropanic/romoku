//! Browser bindings: exposes the core game to JavaScript via wasm-bindgen.
//!
//! JS owns everything platform-specific (canvas drawing, input, timers,
//! camera in pixels); this wrapper only translates between JS-friendly
//! numbers and the core types.
//!
//! Encodings used across the boundary:
//! - player: 1 = X, 2 = O (0 = empty / none)
//! - mode:   index into `Mode::ALL`
//! - lists of positions: flat `[x0, y0, x1, y1, ...]`

use romoku_core::ai;
use romoku_core::app::{App, Controller, Mode, MoveRequest};
use romoku_core::board::Player;
use wasm_bindgen::prelude::*;

fn player_code(p: Player) -> u8 {
    match p {
        Player::X => 1,
        Player::O => 2,
    }
}

#[wasm_bindgen]
pub struct Game {
    app: App,
    /// The CPU request JS has been told about but not yet resolved.
    pending: Option<MoveRequest>,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new(mode: u8) -> Game {
        let mode = Mode::ALL.get(mode as usize).copied().unwrap_or(Mode::HumanVsCpu);
        Game {
            app: App::new(mode),
            pending: None,
        }
    }

    // ---- modes ---------------------------------------------------------

    pub fn mode_count() -> u8 {
        Mode::ALL.len() as u8
    }

    pub fn mode_label(mode: u8) -> String {
        Mode::ALL
            .get(mode as usize)
            .map(|m| m.label().to_string())
            .unwrap_or_default()
    }

    pub fn mode(&self) -> u8 {
        Mode::ALL.iter().position(|&m| m == self.app.mode).unwrap() as u8
    }

    pub fn set_mode(&mut self, mode: u8) {
        if let Some(&m) = Mode::ALL.get(mode as usize) {
            self.app.set_mode(m);
            self.pending = None;
        }
    }

    pub fn restart(&mut self) {
        self.app.restart();
        self.pending = None;
    }

    // ---- state ---------------------------------------------------------

    pub fn turn(&self) -> u8 {
        player_code(self.app.turn)
    }

    pub fn winner(&self) -> u8 {
        self.app.winner.as_ref().map_or(0, |(p, _)| player_code(*p))
    }

    pub fn winning_line(&self) -> Vec<i32> {
        self.app
            .winner
            .as_ref()
            .map(|(_, line)| line.iter().flat_map(|&(x, y)| [x, y]).collect())
            .unwrap_or_default()
    }

    /// All stones as flat `[x, y, player, x, y, player, ...]`.
    pub fn stones(&self) -> Vec<i32> {
        self.app
            .board
            .stones()
            .flat_map(|((x, y), p)| [x, y, player_code(p) as i32])
            .collect()
    }

    /// `[x, y]` of the last move, or empty.
    pub fn last_move(&self) -> Vec<i32> {
        self.app.last_move.map(|(x, y)| vec![x, y]).unwrap_or_default()
    }

    pub fn moves(&self) -> u32 {
        self.app.moves as u32
    }

    pub fn is_human_turn(&self) -> bool {
        self.app.winner.is_none() && self.app.current_controller() == Controller::Human
    }

    pub fn is_waiting(&self) -> bool {
        self.app.is_waiting()
    }

    // ---- cursor (keyboard play) ----------------------------------------

    pub fn cursor_x(&self) -> i32 {
        self.app.cursor.0
    }

    pub fn cursor_y(&self) -> i32 {
        self.app.cursor.1
    }

    pub fn set_cursor(&mut self, x: i32, y: i32) {
        self.app.cursor = (x, y);
    }

    pub fn move_cursor(&mut self, dx: i32, dy: i32) {
        self.app.move_cursor(dx, dy);
    }

    // ---- moves ---------------------------------------------------------

    /// Human move at a cell. Returns false if not allowed.
    pub fn play(&mut self, x: i32, y: i32) -> bool {
        self.app.place_human((x, y))
    }

    /// If the CPU must move, registers the request and returns its ticket.
    /// JS then schedules `resolve_cpu(ticket)` (e.g. via `setTimeout`).
    /// Tickets are u32 on the JS side; they would take years to wrap.
    pub fn request_cpu(&mut self) -> Option<u32> {
        let req = self.app.take_move_request()?;
        let ticket = req.ticket as u32;
        self.pending = Some(req);
        Some(ticket)
    }

    /// Computes and plays the CPU move for `ticket`. Stale tickets
    /// (e.g. from before a restart) do nothing and return false.
    pub fn resolve_cpu(&mut self, ticket: u32) -> bool {
        let Some(req) = self.pending.take_if(|r| r.ticket as u32 == ticket) else {
            return false;
        };
        let pos = ai::best_move(&req.board, req.player);
        self.app.submit_move(req.ticket, pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_then_cpu_round_trip() {
        let mut g = Game::new(0);
        assert!(g.is_human_turn());
        assert!(g.request_cpu().is_none());
        assert!(g.play(0, 0));
        let t = g.request_cpu().expect("cpu to move");
        assert!(g.resolve_cpu(t));
        assert_eq!(g.moves(), 2);
        assert_eq!(g.stones().len(), 6);
    }

    #[test]
    fn stale_ticket_after_restart() {
        let mut g = Game::new(1); // CPU first
        let old = g.request_cpu().unwrap();
        g.restart();
        assert!(!g.resolve_cpu(old));
        let new = g.request_cpu().unwrap();
        assert!(g.resolve_cpu(new));
    }
}
