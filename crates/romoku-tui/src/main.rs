mod ui;

use std::io;
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::{Duration, Instant};

use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};

use romoku_core::ai;
use romoku_core::app::{App, Mode, MoveRequest};
use romoku_core::board::Pos;
use romoku_core::camera::Camera;

/// How often the loop wakes up to check for CPU/remote moves.
const TICK: Duration = Duration::from_millis(30);
/// Minimum time the CPU "thinks", so its move doesn't appear instantly.
const CPU_MIN_DELAY: Duration = Duration::from_millis(300);

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let result = run(&mut terminal);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal) -> io::Result<()> {
    let mut app = App::new(Mode::HumanVsCpu);
    let mut camera = Camera::default();
    // Worker threads send (ticket, move) back here. A network opponent would
    // feed the same channel later.
    let (tx, rx) = mpsc::channel::<(u64, Pos)>();

    loop {
        if let Some(req) = app.take_move_request() {
            spawn_cpu(req, tx.clone());
        }
        while let Ok((ticket, pos)) = rx.try_recv() {
            app.submit_move(ticket, pos); // stale tickets are ignored
        }

        terminal.draw(|frame| ui::draw(frame, &app, &mut camera))?;

        if !event::poll(TICK)? {
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match key.code {
            KeyCode::Left | KeyCode::Char('h') => app.move_cursor(-1, 0),
            KeyCode::Right | KeyCode::Char('l') => app.move_cursor(1, 0),
            KeyCode::Up | KeyCode::Char('k') => app.move_cursor(0, -1),
            KeyCode::Down | KeyCode::Char('j') => app.move_cursor(0, 1),
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.place_at_cursor();
            }
            KeyCode::Char('c') => camera.center = app.cursor,
            KeyCode::Char('n') => app.restart(),
            KeyCode::Char('m') => app.cycle_mode(),
            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
            _ => {}
        }
    }
}

/// Computes the CPU move on a worker thread so the UI never blocks.
fn spawn_cpu(req: MoveRequest, tx: Sender<(u64, Pos)>) {
    thread::spawn(move || {
        let start = Instant::now();
        let pos = ai::best_move(&req.board, req.player);
        thread::sleep(CPU_MIN_DELAY.saturating_sub(start.elapsed()));
        let _ = tx.send((req.ticket, pos));
    });
}
