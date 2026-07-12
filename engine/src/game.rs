//! Game state and simultaneous-turn resolution.
//!
//! This is a deliberately simplified ruleset compared to the original design
//! in `old/README.md` — see `DESIGN.md` at the repo root for the rationale.
//! The key simplifications: orders only move armies to *adjacent* tiles, each
//! player gets a fixed number of orders per turn, and combat resolves in a
//! single deterministic step.

use std::collections::HashMap;

use crate::coords::{hexagon, Direction, Hex};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct PlayerId(pub u8);

#[derive(Clone, Debug)]
pub struct Config {
    /// Map radius; the map has 3r^2 + 3r + 1 tiles.
    pub radius: i32,
    /// 2 to 6 players (one map corner each).
    pub players: u8,
    /// Orders each player may issue per turn (the CP budget, discretized).
    pub orders_per_turn: usize,
    /// Hard turn limit; at the limit the player owning the most tiles wins.
    pub max_turns: u32,
    /// Army on each player's starting tile.
    pub initial_army: u32,
    /// Resources every tile starts with.
    pub initial_resources: u32,
    /// Resources stop growing at this value.
    pub resource_cap: u32,
    /// Per-turn growth is `max(1, (resource_cap - resources) / growth_divisor)`:
    /// fast when low, slow near the cap — the anti-snowball curve.
    pub growth_divisor: u32,
    /// Defender strength multiplier in percent (125 = 1.25x).
    pub defense_bonus_pct: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            radius: 5,
            players: 2,
            orders_per_turn: 3,
            max_turns: 500,
            initial_army: 20,
            initial_resources: 10,
            resource_cap: 100,
            growth_divisor: 25,
            defense_bonus_pct: 125,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Tile {
    pub owner: Option<PlayerId>,
    pub army: u32,
    pub resources: u32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MoveAmount {
    All,
    Half,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Order {
    /// Move armies from an owned tile to an adjacent tile.
    Move {
        from: Hex,
        dir: Direction,
        amount: MoveAmount,
    },
    /// Convert all resources on an owned tile into armies.
    Recruit { at: Hex },
}

impl Order {
    /// The tile this order acts from. A tile can be the source of at most
    /// one order per turn.
    pub fn source(&self) -> Hex {
        match *self {
            Order::Move { from, .. } => from,
            Order::Recruit { at } => at,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Ongoing,
    Winner(PlayerId),
    Draw,
}

#[derive(Clone, Debug)]
pub struct GameState {
    pub config: Config,
    pub turn: u32,
    tiles: Vec<Tile>,
    hexes: Vec<Hex>,
    index: HashMap<Hex, usize>,
}

impl GameState {
    pub fn new(config: Config) -> Self {
        assert!(
            (2..=6).contains(&config.players),
            "2 to 6 players supported (one corner each)"
        );
        assert!(config.radius >= 1, "map too small");
        assert!(config.growth_divisor >= 1, "growth_divisor must be nonzero");

        let hexes: Vec<Hex> = hexagon(config.radius).collect();
        let index: HashMap<Hex, usize> = hexes.iter().enumerate().map(|(i, h)| (*h, i)).collect();
        let tiles = vec![
            Tile {
                owner: None,
                army: 0,
                resources: config.initial_resources,
            };
            hexes.len()
        ];

        let mut state = GameState {
            config,
            turn: 0,
            tiles,
            hexes,
            index,
        };

        // Start each player on a corner of the map. Direction::ALL walks the
        // six corners in ring order, so spacing players `6 / players` corners
        // apart is as even as the hex allows (exactly even for 2, 3 and 6).
        let r = state.config.radius;
        for p in 0..state.config.players {
            let (dx, dy) =
                Direction::ALL[(p as usize * 6 / state.config.players as usize) % 6].delta();
            let corner = Hex::new(dx * r, dy * r);
            let i = state.index[&corner];
            state.tiles[i].owner = Some(PlayerId(p));
            state.tiles[i].army = state.config.initial_army;
        }
        state
    }

    pub fn tile(&self, hex: Hex) -> Option<&Tile> {
        self.index.get(&hex).map(|&i| &self.tiles[i])
    }

    /// Test-only escape hatch for staging scenarios.
    #[cfg(test)]
    pub(crate) fn set_tile(&mut self, hex: Hex, tile: Tile) {
        let i = self.index[&hex];
        self.tiles[i] = tile;
    }

    pub fn iter_tiles(&self) -> impl Iterator<Item = (Hex, &Tile)> {
        self.hexes.iter().copied().zip(self.tiles.iter())
    }

    pub fn tile_count(&self, player: PlayerId) -> usize {
        self.tiles
            .iter()
            .filter(|t| t.owner == Some(player))
            .count()
    }

    pub fn army_total(&self, player: PlayerId) -> u64 {
        self.tiles
            .iter()
            .filter(|t| t.owner == Some(player))
            .map(|t| t.army as u64)
            .sum()
    }

    pub fn outcome(&self) -> Outcome {
        // One pass over the board; everything below derives from the counts.
        let mut counts = vec![0u32; self.config.players as usize];
        for tile in &self.tiles {
            if let Some(PlayerId(p)) = tile.owner {
                counts[p as usize] += 1;
            }
        }

        let alive = counts.iter().filter(|&&c| c > 0).count();
        let leader = counts
            .iter()
            .enumerate()
            .max_by_key(|(_, &c)| c)
            .map(|(p, _)| PlayerId(p as u8))
            .unwrap();
        let leader_count = counts[leader.0 as usize];
        let contested = counts.iter().filter(|&&c| c == leader_count).count() > 1;

        match alive {
            0 => Outcome::Draw,
            1 => Outcome::Winner(leader),
            _ if self.turn >= self.config.max_turns => {
                if contested {
                    Outcome::Draw
                } else {
                    Outcome::Winner(leader)
                }
            }
            _ => Outcome::Ongoing,
        }
    }

    /// Every individually-legal order for the player in the current state.
    pub fn legal_orders(&self, player: PlayerId) -> Vec<Order> {
        let mut orders = Vec::new();
        for (hex, tile) in self.iter_tiles() {
            if tile.owner != Some(player) {
                continue;
            }
            if tile.resources > 0 {
                orders.push(Order::Recruit { at: hex });
            }
            if tile.army == 0 {
                continue;
            }
            for (dir, neighbor) in hex.neighbors() {
                if !neighbor.in_radius(self.config.radius) {
                    continue;
                }
                orders.push(Order::Move {
                    from: hex,
                    dir,
                    amount: MoveAmount::All,
                });
                if tile.army >= 2 {
                    orders.push(Order::Move {
                        from: hex,
                        dir,
                        amount: MoveAmount::Half,
                    });
                }
            }
        }
        orders
    }

    /// Resolve one simultaneous turn. `orders[p]` are player p's orders,
    /// taken in list order until the per-turn budget is spent; illegal
    /// orders (including a second order from the same source tile) are
    /// dropped silently without consuming budget.
    ///
    /// Each player's orders apply to their own tiles only, so the player
    /// processing order is irrelevant: departures and recruits happen
    /// "at once", then everything lands and fights, then resources grow.
    /// One order per tile per turn also means recruited armies defend
    /// immediately but cannot move until the next turn.
    pub fn step(&mut self, orders: &[Vec<Order>]) -> Outcome {
        assert_eq!(orders.len(), self.config.players as usize);

        let mut acted: Vec<bool> = vec![false; self.tiles.len()];
        // arrivals[tile] = per-player armies landing there this turn
        let mut arrivals: HashMap<usize, HashMap<PlayerId, u64>> = HashMap::new();

        for (p, player_orders) in orders.iter().enumerate() {
            let player = PlayerId(p as u8);
            let mut budget = self.config.orders_per_turn;
            for order in player_orders {
                if budget == 0 {
                    break;
                }
                let Some(&src) = self.index.get(&order.source()) else {
                    continue;
                };
                if acted[src] || self.tiles[src].owner != Some(player) {
                    continue;
                }
                match *order {
                    Order::Recruit { .. } => {
                        let tile = &mut self.tiles[src];
                        if tile.resources == 0 {
                            continue;
                        }
                        tile.army += tile.resources;
                        tile.resources = 0;
                    }
                    Order::Move { from, dir, amount } => {
                        let Some(&dst) = self.index.get(&from.neighbor(dir)) else {
                            continue;
                        };
                        let army = self.tiles[src].army;
                        let moving = match amount {
                            MoveAmount::All => army,
                            MoveAmount::Half => army / 2,
                        };
                        if moving == 0 {
                            continue;
                        }
                        self.tiles[src].army -= moving;
                        *arrivals.entry(dst).or_default().entry(player).or_default() +=
                            moving as u64;
                    }
                }
                acted[src] = true;
                budget -= 1;
            }
        }

        let defense_bonus_pct = self.config.defense_bonus_pct as u64;
        for (dst, mut parties) in arrivals {
            let tile = &mut self.tiles[dst];
            // Neutral tiles never hold armies (ownership never reverts), so
            // there is no "neutral garrison" party to account for.
            debug_assert!(tile.owner.is_some() || tile.army == 0);

            // The defender's party is the garrison plus any same-owner
            // arrivals; the whole party gets the defense bonus.
            if let Some(owner) = tile.owner {
                *parties.entry(owner).or_default() += tile.army as u64;
            }
            let defender = tile.owner;

            if parties.len() == 1 {
                let (player, amount) = parties.into_iter().next().unwrap();
                tile.owner = Some(player);
                tile.army = amount.min(u32::MAX as u64) as u32;
                continue;
            }

            // Deterministic single-step combat: the strongest party survives,
            // paying the combined effective strength of everyone else.
            let effective = |p: PlayerId, actual: u64| -> u64 {
                if Some(p) == defender {
                    actual * defense_bonus_pct / 100
                } else {
                    actual
                }
            };
            let mut ranked: Vec<(PlayerId, u64, u64)> = parties
                .iter()
                .map(|(&p, &a)| (p, a, effective(p, a)))
                .collect();
            // Strongest first; player id breaks ties deterministically.
            ranked.sort_by_key(|&(p, _, eff)| (std::cmp::Reverse(eff), p));

            let (winner, winner_actual, winner_eff) = ranked[0];
            let losses_eff: u64 = ranked[1..].iter().map(|&(_, _, eff)| eff).sum();

            if winner_eff <= losses_eff {
                // Mutual annihilation (covers exact ties for first place,
                // since the runner-up alone already matches winner_eff):
                // tile keeps its owner but is left undefended.
                tile.army = 0;
                continue;
            }
            // Convert the winner's surviving effective strength back to
            // actual units (u128: the product can exceed u64 for huge armies).
            let surviving =
                (winner_eff - losses_eff) as u128 * winner_actual as u128 / winner_eff as u128;
            tile.owner = Some(winner);
            tile.army = surviving.min(u32::MAX as u128) as u32;
        }

        // Resource growth on every tile: fast when poor, slow near the cap.
        for tile in &mut self.tiles {
            if tile.resources < self.config.resource_cap {
                let growth = ((self.config.resource_cap - tile.resources)
                    / self.config.growth_divisor)
                    .max(1);
                tile.resources = (tile.resources + growth).min(self.config.resource_cap);
            }
        }

        self.turn += 1;
        self.outcome()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_config() -> Config {
        Config {
            radius: 3,
            ..Config::default()
        }
    }

    fn state() -> GameState {
        GameState::new(small_config())
    }

    fn start_of(s: &GameState, player: PlayerId) -> Hex {
        s.iter_tiles()
            .find(|(_, t)| t.owner == Some(player))
            .unwrap()
            .0
    }

    fn some_in_map_direction(s: &GameState, from: Hex) -> Direction {
        Direction::ALL
            .into_iter()
            .find(|d| from.neighbor(*d).in_radius(s.config.radius))
            .unwrap()
    }

    /// Plant a player-1 tile with the given army next to player 0's start;
    /// returns (defended hex, attacker hex, direction attacker -> defender).
    fn stage_attack(s: &mut GameState, attacker_army: u32) -> (Hex, Hex, Direction) {
        let p0_start = start_of(s, PlayerId(0));
        let out = some_in_map_direction(s, p0_start);
        let attacker_hex = p0_start.neighbor(out);
        s.set_tile(
            attacker_hex,
            Tile {
                owner: Some(PlayerId(1)),
                army: attacker_army,
                resources: 0,
            },
        );
        let back = Direction::ALL
            .into_iter()
            .find(|d| attacker_hex.neighbor(*d) == p0_start)
            .unwrap();
        (p0_start, attacker_hex, back)
    }

    fn move_all(from: Hex, dir: Direction) -> Order {
        Order::Move {
            from,
            dir,
            amount: MoveAmount::All,
        }
    }

    #[test]
    fn initial_state_is_symmetric() {
        let s = state();
        let p0 = PlayerId(0);
        let p1 = PlayerId(1);
        assert_eq!(s.tile_count(p0), 1);
        assert_eq!(s.tile_count(p1), 1);
        assert_eq!(s.army_total(p0), s.army_total(p1));
        // Two players start on opposite corners.
        assert_eq!(
            start_of(&s, p0).distance(start_of(&s, p1)),
            2 * s.config.radius
        );
        assert_eq!(s.outcome(), Outcome::Ongoing);
    }

    #[test]
    fn players_get_distinct_corners() {
        for players in 2..=6 {
            let s = GameState::new(Config {
                players,
                ..small_config()
            });
            for p in 0..players {
                assert_eq!(s.tile_count(PlayerId(p)), 1, "players={players} p={p}");
            }
        }
    }

    #[test]
    fn move_transfers_army_to_empty_tile() {
        let mut s = state();
        let start = start_of(&s, PlayerId(0));
        let dir = some_in_map_direction(&s, start);
        let dest = start.neighbor(dir);

        s.step(&[
            vec![Order::Move {
                from: start,
                dir,
                amount: MoveAmount::Half,
            }],
            vec![],
        ]);

        let initial = s.config.initial_army;
        assert_eq!(s.tile(start).unwrap().army, initial - initial / 2);
        assert_eq!(s.tile(dest).unwrap().army, initial / 2);
        assert_eq!(s.tile(dest).unwrap().owner, Some(PlayerId(0)));
        assert_eq!(s.tile_count(PlayerId(0)), 2);
    }

    #[test]
    fn recruit_converts_resources() {
        let mut s = state();
        let start = start_of(&s, PlayerId(0));
        let tile = *s.tile(start).unwrap();
        assert!(tile.resources > 0);

        s.step(&[vec![Order::Recruit { at: start }], vec![]]);

        let after = s.tile(start).unwrap();
        assert_eq!(after.army, tile.army + tile.resources);
        // growth phase runs after recruiting, so resources are small but non-zero
        assert!(after.resources > 0);
        assert!(after.resources < tile.resources);
    }

    #[test]
    fn one_order_per_tile_recruited_armies_cannot_move() {
        let mut s = state();
        let start = start_of(&s, PlayerId(0));
        let dir = some_in_map_direction(&s, start);
        let before = *s.tile(start).unwrap();

        // Recruit then move from the same tile: the move must be dropped,
        // so the freshly recruited armies stay put this turn.
        s.step(&[
            vec![Order::Recruit { at: start }, move_all(start, dir)],
            vec![],
        ]);

        assert_eq!(
            s.tile(start).unwrap().army,
            before.army + before.resources,
            "recruited armies moved on the turn they were raised"
        );
        assert_eq!(s.tile(start.neighbor(dir)).unwrap().army, 0);
    }

    #[test]
    fn budget_respects_submitted_order() {
        let mut s = GameState::new(Config {
            orders_per_turn: 1,
            ..small_config()
        });
        let start = start_of(&s, PlayerId(0));
        let dir = some_in_map_direction(&s, start);
        // Give player 0 a second tile so two distinct sources exist.
        let second = start.neighbor(dir);
        s.set_tile(
            second,
            Tile {
                owner: Some(PlayerId(0)),
                army: 0,
                resources: 40,
            },
        );

        // Budget 1, [Move from start, Recruit at second]: the move is first
        // in the list, so it executes and the recruit is dropped.
        s.step(&[
            vec![move_all(start, dir), Order::Recruit { at: second }],
            vec![],
        ]);

        assert_eq!(
            s.tile(start).unwrap().army,
            0,
            "listed-first move must execute"
        );
        assert_eq!(
            s.tile(second).unwrap().army,
            s.config.initial_army,
            "recruit must be dropped once the budget is spent"
        );
    }

    #[test]
    fn defender_with_bonus_repels_equal_attack() {
        let mut s = state();
        let initial = s.config.initial_army;
        let (p0_start, attacker_hex, back) = stage_attack(&mut s, initial);

        s.step(&[vec![], vec![move_all(attacker_hex, back)]]);

        // Equal armies, but the defender's 1.25x bonus wins the fight.
        let defended = s.tile(p0_start).unwrap();
        assert_eq!(defended.owner, Some(PlayerId(0)));
        assert!(defended.army > 0);
        assert!(defended.army < s.config.initial_army);
    }

    #[test]
    fn overwhelming_attack_conquers() {
        let mut s = state();
        let initial = s.config.initial_army;
        let (p0_start, attacker_hex, back) = stage_attack(&mut s, initial * 10);

        s.step(&[vec![], vec![move_all(attacker_hex, back)]]);

        let taken = s.tile(p0_start).unwrap();
        assert_eq!(taken.owner, Some(PlayerId(1)));
        assert!(taken.army > 0);
        // Player 0 lost their only tile: game over.
        assert_eq!(s.outcome(), Outcome::Winner(PlayerId(1)));
    }

    #[test]
    fn huge_armies_do_not_overflow_combat() {
        let mut s = state();
        let (p0_start, attacker_hex, back) = stage_attack(&mut s, u32::MAX);
        s.set_tile(
            p0_start,
            Tile {
                owner: Some(PlayerId(0)),
                army: u32::MAX / 2,
                resources: 0,
            },
        );

        s.step(&[vec![], vec![move_all(attacker_hex, back)]]);

        let taken = s.tile(p0_start).unwrap();
        assert_eq!(taken.owner, Some(PlayerId(1)));
        assert!(taken.army > 0);
        assert!(taken.army < u32::MAX);
    }

    #[test]
    fn resources_grow_toward_cap() {
        let mut s = state();
        for _ in 0..2000 {
            s.step(&[vec![], vec![]]);
            if s.turn >= s.config.max_turns {
                break;
            }
        }
        let cap = s.config.resource_cap;
        assert!(s.iter_tiles().all(|(_, t)| t.resources == cap));
    }

    #[test]
    fn random_games_terminate_and_stay_consistent() {
        use crate::rng::Rng;
        let mut rng = Rng::new(42);
        for _ in 0..5 {
            let mut s = GameState::new(Config {
                max_turns: 200,
                ..small_config()
            });
            loop {
                let orders: Vec<Vec<Order>> = (0..s.config.players)
                    .map(|p| {
                        let legal = s.legal_orders(PlayerId(p));
                        (0..s.config.orders_per_turn)
                            .filter_map(|_| {
                                if legal.is_empty() {
                                    None
                                } else {
                                    Some(legal[rng.below(legal.len())])
                                }
                            })
                            .collect()
                    })
                    .collect();
                if s.step(&orders) != Outcome::Ongoing {
                    break;
                }
                assert!(s.turn <= 200, "game must terminate by max_turns");
            }
        }
    }
}
