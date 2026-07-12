//! Playable macroquad frontend: a human (player A) versus a scripted bot.
//!
//! The same crate builds to native desktop, web (wasm32) and Android;
//! see `app/README.md` for the per-target build commands.
//! All game logic comes from `overthrow-engine`;
//! this file is input, layout and drawing only.
//!
//! Controls: left-click one of your tiles to select it,
//! then left-click an adjacent tile to queue a move
//! (hold Shift for a half-move);
//! right-click a tile (or press R while it's selected) to queue a recruit.
//! Backspace undoes the last queued order, Enter ends the turn,
//! N starts a new game.

mod layout;

use layout::Layout;
use macroquad::prelude::*;
use overthrow_bot::{make_bot, Bot};
use overthrow_engine::{
    coords::Direction, Config, GameState, Hex, MoveAmount, Order, Outcome, PlayerId,
};

const HUMAN: PlayerId = PlayerId(0);
const BOT_NAME: &str = "greedy";

/// Distinct fill color per player id (index by `PlayerId.0`).
const PLAYER_COLORS: [Color; 6] = [
    Color::new(0.26, 0.53, 0.96, 1.0), // A — blue (human)
    Color::new(0.90, 0.30, 0.24, 1.0), // B — red
    Color::new(0.30, 0.72, 0.40, 1.0), // C — green
    Color::new(0.95, 0.65, 0.15, 1.0), // D — orange
    Color::new(0.65, 0.40, 0.85, 1.0), // E — purple
    Color::new(0.20, 0.75, 0.78, 1.0), // F — teal
];
const NEUTRAL: Color = Color::new(0.22, 0.24, 0.28, 1.0);
const BG: Color = Color::new(0.09, 0.10, 0.12, 1.0);

struct App {
    state: GameState,
    bot: Box<dyn Bot>,
    /// Human orders staged for the current turn (source-unique, budget-capped).
    pending: Vec<Order>,
    selected: Option<Hex>,
    seed: u64,
}

impl App {
    fn new(seed: u64) -> Self {
        App {
            state: GameState::new(Config::default()),
            bot: make_bot(BOT_NAME, seed.wrapping_mul(2) + 1).expect("known bot"),
            pending: Vec::new(),
            selected: None,
            seed,
        }
    }

    fn restart(&mut self) {
        let next = self.seed.wrapping_add(1);
        *self = App::new(next);
    }

    /// True while the human may still add orders (game live, budget left).
    fn accepting_orders(&self) -> bool {
        self.state.outcome() == Outcome::Ongoing
            && self.pending.len() < self.state.config.orders_per_turn
    }

    /// Whether `hex` is already the source of a queued order
    /// (the engine allows at most one order per tile per turn).
    fn source_taken(&self, hex: Hex) -> bool {
        self.pending.iter().any(|o| o.source() == hex)
    }

    fn queue(&mut self, order: Order) {
        if self.accepting_orders() && !self.source_taken(order.source()) {
            self.pending.push(order);
        }
    }

    /// Resolve one turn: human's staged orders plus the bot's, then advance.
    fn end_turn(&mut self) {
        if self.state.outcome() != Outcome::Ongoing {
            return;
        }
        let bot_orders = self.bot.orders(&self.state, PlayerId(1));
        self.state
            .step(&[std::mem::take(&mut self.pending), bot_orders]);
        self.selected = None;
    }

    fn handle_click(&mut self, hex: Hex) {
        if !hex.in_radius(self.state.config.radius) {
            self.selected = None;
            return;
        }
        let mine = self.state.tile(hex).and_then(|t| t.owner) == Some(HUMAN);

        // Clicking an adjacent tile of the current selection issues a move.
        if let Some(src) = self.selected {
            if let Some(dir) = direction_between(src, hex) {
                let amount = if shift_down() {
                    MoveAmount::Half
                } else {
                    MoveAmount::All
                };
                self.queue(Order::Move {
                    from: src,
                    dir,
                    amount,
                });
                self.selected = None;
                return;
            }
        }

        // Otherwise (re)select one of our own tiles, or clear.
        self.selected = mine.then_some(hex);
    }

    /// Queue a recruit on `at`,
    /// if it is one of our tiles with resources to convert.
    fn recruit_at(&mut self, at: Hex) {
        let ours = self
            .state
            .tile(at)
            .is_some_and(|t| t.owner == Some(HUMAN) && t.resources > 0);
        if ours {
            self.queue(Order::Recruit { at });
            self.selected = None;
        }
    }

    /// Dispatch this frame's mouse and keyboard input onto the game.
    fn handle_input(&mut self, layout: &Layout, hud_h: f32) {
        if is_key_pressed(KeyCode::N) {
            self.restart();
        }
        if self.state.outcome() != Outcome::Ongoing {
            return;
        }
        let (mx, my) = mouse_position();
        let on_map = my > hud_h;
        if is_mouse_button_pressed(MouseButton::Left) && on_map {
            self.handle_click(layout.hex_at(mx, my));
        }
        if is_mouse_button_pressed(MouseButton::Right) && on_map {
            self.recruit_at(layout.hex_at(mx, my));
        }
        if is_key_pressed(KeyCode::R) {
            if let Some(hex) = self.selected {
                self.recruit_at(hex);
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            self.pending.pop();
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            self.end_turn();
        }
    }
}

/// The direction stepping `from` to `to`, if they are adjacent map hexes.
fn direction_between(from: Hex, to: Hex) -> Option<Direction> {
    Direction::ALL.into_iter().find(|&d| from.neighbor(d) == to)
}

fn shift_down() -> bool {
    is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift)
}

/// Fit the whole map into the drawable area below the HUD, centered.
fn fit_layout(state: &GameState, hud_h: f32) -> Layout {
    // Bounding box of all centers at unit size.
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
    let unit = Layout {
        size: 1.0,
        origin: (0.0, 0.0),
    };
    for (hex, _) in state.iter_tiles() {
        let (x, y) = unit.center(hex);
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    // +2 unit cells of padding around the extent so edge hexes aren't clipped.
    let span_x = (max_x - min_x) + 2.0;
    let span_y = (max_y - min_y) + 2.0;
    let avail_w = screen_width() - 24.0;
    let avail_h = screen_height() - hud_h - 24.0;
    let size = (avail_w / span_x).min(avail_h / span_y).max(4.0);
    let cx = 12.0 + avail_w / 2.0 - (min_x + max_x) / 2.0 * size;
    let cy = hud_h + 12.0 + avail_h / 2.0 - (min_y + max_y) / 2.0 * size;
    Layout {
        size,
        origin: (cx, cy),
    }
}

fn draw_tile(state: &GameState, layout: &Layout, hex: Hex, highlight: TileHighlight) {
    let tile = state.tile(hex).expect("in-map hex");
    let (cx, cy) = layout.center(hex);
    let radius = layout.size * 0.92;
    let fill = match tile.owner {
        Some(PlayerId(p)) => PLAYER_COLORS[p as usize % PLAYER_COLORS.len()],
        None => NEUTRAL,
    };
    draw_poly(cx, cy, 6, radius, 30.0, fill);

    let (border_color, border_w) = match highlight {
        TileHighlight::Selected => (YELLOW, 3.5),
        TileHighlight::MoveTarget => (GREEN, 3.0),
        TileHighlight::None => (Color::new(0.0, 0.0, 0.0, 0.5), 1.5),
    };
    draw_poly_lines(cx, cy, 6, radius, 30.0, border_w, border_color);

    // Army count (bold, centered) and a small resources readout beneath.
    if tile.owner.is_some() || tile.army > 0 {
        centered_text(
            &tile.army.to_string(),
            cx,
            cy - layout.size * 0.05,
            layout.size * 0.9,
            WHITE,
        );
    }
    let res = format!("r{}", tile.resources);
    centered_text(
        &res,
        cx,
        cy + layout.size * 0.55,
        layout.size * 0.5,
        Color::new(1.0, 1.0, 1.0, 0.55),
    );
}

#[derive(Clone, Copy, PartialEq)]
enum TileHighlight {
    None,
    Selected,
    MoveTarget,
}

fn centered_text(text: &str, cx: f32, cy: f32, font_size: f32, color: Color) {
    let fs = font_size.max(8.0) as u16;
    let dims = measure_text(text, None, fs, 1.0);
    draw_text(
        text,
        cx - dims.width / 2.0,
        cy + dims.height / 2.0,
        fs as f32,
        color,
    );
}

fn draw_pending(layout: &Layout, pending: &[Order]) {
    for order in pending {
        match *order {
            Order::Move { from, dir, amount } => {
                let (sx, sy) = layout.center(from);
                let (tx, ty) = layout.center(from.neighbor(dir));
                let col = if amount == MoveAmount::Half {
                    ORANGE
                } else {
                    GOLD
                };
                draw_line(sx, sy, tx, ty, 4.0, col);
                // A dot marks the destination end of the move.
                draw_circle(tx, ty, layout.size * 0.14, col);
            }
            Order::Recruit { at } => {
                let (x, y) = layout.center(at);
                draw_poly_lines(x, y, 6, layout.size * 0.6, 30.0, 3.0, GOLD);
                centered_text("+", x, y - layout.size * 0.05, layout.size * 0.7, GOLD);
            }
        }
    }
}

fn draw_hud(app: &App) -> f32 {
    let hud_h = 88.0;
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        hud_h,
        Color::new(0.0, 0.0, 0.0, 0.35),
    );

    let s = &app.state;
    let you = s.tile_count(HUMAN);
    let foe = s.tile_count(PlayerId(1));
    let line1 = format!(
        "Turn {}/{}    You (A): {} tiles, {} army    {} (B): {} tiles, {} army",
        s.turn,
        s.config.max_turns,
        you,
        s.army_total(HUMAN),
        BOT_NAME,
        foe,
        s.army_total(PlayerId(1)),
    );
    draw_text(&line1, 14.0, 26.0, 22.0, WHITE);

    let line2 = format!(
        "Orders {}/{}    Shift+click = half-move",
        app.pending.len(),
        s.config.orders_per_turn,
    );
    draw_text(&line2, 14.0, 50.0, 18.0, Color::new(1.0, 1.0, 1.0, 0.8));

    let hint = "L-click: select / move   R-click or R: recruit   Backspace: undo   Enter: end turn   N: new game";
    draw_text(hint, 14.0, 72.0, 16.0, Color::new(1.0, 1.0, 1.0, 0.6));

    // Outcome banner when the game is over.
    match s.outcome() {
        Outcome::Ongoing => {}
        result => {
            let msg = match result {
                Outcome::Winner(HUMAN) => "You win!  —  press N for a new game".to_string(),
                Outcome::Winner(PlayerId(p)) => {
                    format!(
                        "Player {} wins  —  press N for a new game",
                        (b'A' + p) as char
                    )
                }
                Outcome::Draw => "Draw  —  press N for a new game".to_string(),
                Outcome::Ongoing => unreachable!(),
            };
            let fs = 34.0;
            let dims = measure_text(&msg, None, fs as u16, 1.0);
            let x = screen_width() / 2.0 - dims.width / 2.0;
            let y = screen_height() / 2.0;
            draw_rectangle(
                x - 20.0,
                y - 34.0,
                dims.width + 40.0,
                56.0,
                Color::new(0.0, 0.0, 0.0, 0.75),
            );
            draw_text(&msg, x, y, fs, WHITE);
        }
    }
    hud_h
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Overthrow".to_string(),
        window_width: 900,
        window_height: 760,
        high_dpi: true,
        ..Default::default()
    }
}

/// Draw the board: each tile,
/// with the selection and its adjacent move targets rimmed,
/// then the queued orders on top.
fn draw_map(app: &App, layout: &Layout) {
    for (hex, _) in app.state.iter_tiles() {
        let hl = if Some(hex) == app.selected {
            TileHighlight::Selected
        } else if app
            .selected
            .and_then(|src| direction_between(src, hex))
            .is_some()
        {
            TileHighlight::MoveTarget
        } else {
            TileHighlight::None
        };
        draw_tile(&app.state, layout, hex, hl);
    }
    draw_pending(layout, &app.pending);
}

#[macroquad::main(window_conf)]
async fn main() {
    // Seed is fixed (no wall-clock dependency, matching the engine's style);
    // restarting advances it so successive games differ.
    let mut app = App::new(1);
    loop {
        clear_background(BG);
        let hud_h = draw_hud(&app);
        let layout = fit_layout(&app.state, hud_h);
        app.handle_input(&layout, hud_h);
        draw_map(&app, &layout);
        next_frame().await;
    }
}
