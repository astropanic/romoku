use ratatui::{
    Frame,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget},
};

use romoku_core::app::{App, Controller};
use romoku_core::board::Player;
use romoku_core::camera::{Camera, View};

/// Each board cell is drawn 2 terminal columns wide so the grid looks square.
const CELL_WIDTH: u16 = 2;
/// How close (in cells) the cursor may get to the edge before the camera scrolls.
const SCROLL_MARGIN: i32 = 3;

pub fn draw(frame: &mut Frame, app: &App, camera: &mut Camera) {
    let [board_area, status_area] =
        Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(frame.area());

    let block = Block::bordered().title(" romoku ");
    let inner = block.inner(board_area);
    frame.render_widget(block, board_area);

    let (w, h) = ((inner.width / CELL_WIDTH) as i32, inner.height as i32);
    if w > 0 && h > 0 {
        // The camera only knows the viewport size at render time, so follow here.
        camera.follow(app.cursor, w, h, SCROLL_MARGIN);
        let view = camera.view(w, h);
        frame.render_widget(BoardView { app, view }, inner);
    }

    frame.render_widget(status_line(app), status_area);
}

fn player_style(p: Player) -> Style {
    match p {
        Player::X => Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
        Player::O => Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    }
}

fn player_symbol(p: Player) -> &'static str {
    match p {
        Player::X => "X",
        Player::O => "O",
    }
}

struct BoardView<'a> {
    app: &'a App,
    view: View,
}

impl Widget for BoardView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let app = self.app;
        let win_line = app.winner.as_ref().map(|(_, l)| l.as_slice()).unwrap_or(&[]);
        let empty = Style::new().fg(Color::DarkGray);

        for row in 0..self.view.height {
            for col in 0..self.view.width {
                let pos = (self.view.left + col, self.view.top + row);
                let (symbol, mut style) = match app.board.get(pos) {
                    Some(p) => (player_symbol(p), player_style(p)),
                    None if pos == (0, 0) => ("+", empty),
                    None => ("·", empty),
                };
                if Some(pos) == app.last_move {
                    style = style.add_modifier(Modifier::UNDERLINED);
                }
                if win_line.contains(&pos) {
                    style = style.bg(Color::Yellow).fg(Color::Black);
                }
                if pos == app.cursor {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                let x = area.x + col as u16 * CELL_WIDTH;
                let y = area.y + row as u16;
                buf.set_string(x, y, symbol, style);
            }
        }
    }
}

fn status_line(app: &App) -> Paragraph<'static> {
    let mut spans = vec![Span::styled(
        format!("{} │ ", app.mode.label()),
        Style::new().fg(Color::Gray),
    )];
    match &app.winner {
        Some((p, _)) => spans.extend([
            Span::styled(player_symbol(*p), player_style(*p)),
            Span::raw(" wins!  "),
            Span::styled("n", Style::new().bold()),
            Span::raw(" new game  "),
        ]),
        None => {
            let who = match app.current_controller() {
                Controller::Cpu if app.is_waiting() => " CPU thinking…",
                Controller::Cpu => " CPU",
                Controller::Human => "",
            };
            spans.extend([
                Span::raw("Turn: "),
                Span::styled(player_symbol(app.turn), player_style(app.turn)),
                Span::raw(format!("{who}  ")),
            ]);
        }
    }
    spans.push(Span::styled(
        format!(
            "({}, {})  move {}  │ arrows/hjkl · Enter place · c center · n new · m mode · q quit",
            app.cursor.0, app.cursor.1, app.moves
        ),
        Style::new().fg(Color::Gray),
    ));
    Paragraph::new(Line::from(spans))
}
