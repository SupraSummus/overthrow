//! Playable macroquad frontend: a human plays a bot, or two bots play
//! each other while you watch.
//!
//! The same crate builds to native desktop, web (wasm32) and Android;
//! see `app/README.md` for the per-target build commands.
//! All game logic comes from `overthrow-engine`;
//! this file is input, layout and drawing only.
//!
//! The app runs in one of two modes:
//! *play* (a human is player A, a bot plays B) and
//! *spectate* (both players are bots and the game plays itself,
//! for watching a bot-vs-bot match).
//! `H` starts a fresh play game, `B` a fresh bot-vs-bot game,
//! `N` restarts the current mode; these work in either mode.
//!
//! Play controls: left-click one of your tiles to select it,
//! then left-click an adjacent tile to queue a move
//! (hold Shift for a half-move);
//! right-click a tile (or press R while it's selected) to queue a recruit.
//! Backspace undoes the last queued order, Enter ends the turn.
//!
//! Spectate controls: Space pauses/resumes the auto-advancing match,
//! `.` steps a single turn, and `[` / `]` slow down / speed up the pace.

mod layout;

use layout::Layout;
use macroquad::prelude::*;
use overthrow_bot::{make_bot, Bot};
use overthrow_engine::{
    coords::Direction, Config, GameState, Hex, MoveAmount, Order, Outcome, PlayerId,
};

const HUMAN: PlayerId = PlayerId(0);
/// Bot the human faces in play mode (player B).
const BOT_NAME: &str = "greedy";
/// The two bots pitted against each other in spectate mode (players A, B).
const SPECTATE_BOTS: [&str; 2] = ["greedy", "greedy"];
/// Seconds each spectated turn is held on screen before the next resolves;
/// `[` / `]` scale this within [`TURN_INTERVAL_RANGE`].
const DEFAULT_TURN_INTERVAL: f32 = 0.6;
const TURN_INTERVAL_RANGE: (f32, f32) = (0.05, 2.0);

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
    /// One slot per player; `None` is the human (player A in play mode),
    /// `Some` a bot. A bot in slot A means we are spectating a bot-vs-bot game.
    bots: [Option<Box<dyn Bot>>; 2],
    /// Human orders staged for the current turn (source-unique, CP-capped).
    pending: Vec<Order>,
    selected: Option<Hex>,
    seed: u64,
    /// Spectate pacing: the match is held when `paused`, and auto-advances
    /// once `timer` reaches `turn_interval` (all inert in play mode).
    paused: bool,
    timer: f32,
    turn_interval: f32,
}

impl App {
    fn new(seed: u64, spectate: bool) -> Self {
        // Distinct sub-seeds per bot so two greedies don't play in lockstep.
        let bots = if spectate {
            [
                Some(make_bot(SPECTATE_BOTS[0], seed.wrapping_mul(2)).expect("known bot")),
                Some(make_bot(SPECTATE_BOTS[1], seed.wrapping_mul(2) + 1).expect("known bot")),
            ]
        } else {
            [
                None,
                Some(make_bot(BOT_NAME, seed.wrapping_mul(2) + 1).expect("known bot")),
            ]
        };
        App {
            state: GameState::new(Config::default()),
            bots,
            pending: Vec::new(),
            selected: None,
            seed,
            paused: false,
            timer: 0.0,
            turn_interval: DEFAULT_TURN_INTERVAL,
        }
    }

    /// Restart in the same mode with the next seed, so successive games differ.
    fn restart(&mut self) {
        *self = App::new(self.seed.wrapping_add(1), self.spectating());
    }

    /// True when player A is a bot, i.e. we are watching a bot-vs-bot match.
    fn spectating(&self) -> bool {
        self.bots[HUMAN.0 as usize].is_some()
    }

    /// Display name for player `p`: the bot's name, or "you" for the human.
    fn player_label(&self, p: usize) -> &'static str {
        match &self.bots[p] {
            Some(bot) => bot.name(),
            None => "you",
        }
    }

    /// Command points the staged orders will spend (see `GameState::step`).
    fn pending_cp(&self) -> u32 {
        self.pending.iter().map(|o| self.state.order_cost(o)).sum()
    }

    /// True while the human may still add orders (game live, CP left).
    fn accepting_orders(&self) -> bool {
        self.state.outcome() == Outcome::Ongoing
            && self.pending_cp() < self.state.config.command_points
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

    /// Resolve one turn: each player's orders (a bot's, or the human's staged
    /// pending in play mode), then advance and reset the spectate timer.
    fn advance_turn(&mut self) {
        if self.state.outcome() != Outcome::Ongoing {
            return;
        }
        let mut orders: Vec<Vec<Order>> = Vec::with_capacity(self.bots.len());
        for (p, slot) in self.bots.iter_mut().enumerate() {
            orders.push(match slot {
                Some(bot) => bot.orders(&self.state, PlayerId(p as u8)),
                None => std::mem::take(&mut self.pending),
            });
        }
        self.state.step(&orders);
        self.selected = None;
        self.timer = 0.0;
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
        // Mode / new-game keys work in either mode and any game state.
        if is_key_pressed(KeyCode::N) {
            self.restart();
            return;
        }
        if is_key_pressed(KeyCode::B) {
            *self = App::new(self.seed.wrapping_add(1), true);
            return;
        }
        if is_key_pressed(KeyCode::H) {
            *self = App::new(self.seed.wrapping_add(1), false);
            return;
        }
        if self.spectating() {
            self.handle_spectate_input();
        } else {
            self.handle_play_input(layout, hud_h);
        }
    }

    /// Play mode: mouse selection/moves, recruit, undo, end turn.
    fn handle_play_input(&mut self, layout: &Layout, hud_h: f32) {
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
            self.advance_turn();
        }
    }

    /// Spectate mode: pause/resume, single-step, and pace control.
    fn handle_spectate_input(&mut self) {
        if is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Enter) {
            self.paused = !self.paused;
        }
        if is_key_pressed(KeyCode::Period) {
            self.advance_turn();
        }
        if is_key_pressed(KeyCode::LeftBracket) {
            self.turn_interval = (self.turn_interval * 1.5).min(TURN_INTERVAL_RANGE.1);
        }
        if is_key_pressed(KeyCode::RightBracket) {
            self.turn_interval = (self.turn_interval / 1.5).max(TURN_INTERVAL_RANGE.0);
        }
    }

    /// Advance a spectated match on its own clock; a no-op in play mode,
    /// when paused, or once the game is over.
    fn tick(&mut self, dt: f32) {
        if !self.spectating() || self.paused || self.state.outcome() != Outcome::Ongoing {
            return;
        }
        self.timer += dt;
        if self.timer >= self.turn_interval {
            self.advance_turn(); // resets the timer
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
    let line1 = format!(
        "Turn {}/{}    A [{}]: {} tiles, {} army    B [{}]: {} tiles, {} army",
        s.turn,
        s.config.max_turns,
        app.player_label(0),
        s.tile_count(PlayerId(0)),
        s.army_total(PlayerId(0)),
        app.player_label(1),
        s.tile_count(PlayerId(1)),
        s.army_total(PlayerId(1)),
    );
    draw_text(&line1, 14.0, 26.0, 22.0, WHITE);

    let dim = Color::new(1.0, 1.0, 1.0, 0.8);
    let dimmer = Color::new(1.0, 1.0, 1.0, 0.6);
    let (line2, hint) = if app.spectating() {
        let status = if s.outcome() != Outcome::Ongoing {
            "game over".to_string()
        } else if app.paused {
            "paused".to_string()
        } else {
            format!("playing, {:.2}s/turn", app.turn_interval)
        };
        (
            format!("Bot vs bot — {status}"),
            "Space: pause   .: step   [ ]: slower/faster   H: play vs bot   N: new match",
        )
    } else {
        (
            format!(
                "CP {}/{}    Shift+click = half-move",
                app.pending_cp(),
                s.config.command_points,
            ),
            "L-click: select / move   R-click or R: recruit   Backspace: undo   Enter: end turn   N: new   B: watch bots",
        )
    };
    draw_text(&line2, 14.0, 50.0, 18.0, dim);
    draw_text(hint, 14.0, 72.0, 16.0, dimmer);

    // Outcome banner when the game is over.
    match s.outcome() {
        Outcome::Ongoing => {}
        result => {
            let msg = match result {
                Outcome::Winner(HUMAN) if !app.spectating() => {
                    "You win!  —  press N for a new game".to_string()
                }
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
    let mut app = App::new(1, false);
    loop {
        clear_background(BG);
        let hud_h = draw_hud(&app);
        let layout = fit_layout(&app.state, hud_h);
        app.handle_input(&layout, hud_h);
        app.tick(get_frame_time());
        draw_map(&app, &layout);
        next_frame().await;
    }
}
