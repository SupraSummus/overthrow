//! Game state and simultaneous-turn resolution.
//!
//! This is a deliberately simplified ruleset
//! compared to the original design in `old/README.md` —
//! see `DESIGN.md` at the repo root for the rationale.
//! The key simplifications:
//! orders only move armies to *adjacent* tiles,
//! each player spends a fixed pool of command points per turn
//! (one per army moved or raised, no accumulation),
//! and combat resolves in a single deterministic step.

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
    /// Command points each player may spend per turn,
    /// with no accumulation between turns.
    /// A `Move` spends one CP per army moved;
    /// a `Recruit` spends one CP per army raised.
    /// Orders are funded in submission order until the pool runs out;
    /// the order that can only be paid in part
    /// runs partially and empties the pool (see `GameState::step`).
    pub command_points: u32,
    /// Hard turn limit; at the limit the player owning the most tiles wins.
    pub max_turns: u32,
    /// Army on each player's starting tile.
    pub initial_army: u32,
    /// Resources every tile starts with.
    pub initial_resources: u32,
    /// Flat resources every tile produces each turn — the production half of
    /// the production/maintenance flow (see `maintenance_pct`).
    pub production: u32,
    /// Per-turn maintenance each tile pays,
    /// as a percent of the resources present (`2` = 2% of the stockpile).
    /// Production minus this maintenance is a self-limiting flow:
    /// a poor tile grows fast and a near-full one barely grows
    /// — the anti-snowball curve —
    /// settling at the emergent cap `Config::resource_equilibrium`.
    /// A percentage, so it must be in `1..=100`.
    pub maintenance_pct: u32,
}

impl Config {
    /// The resource level a tile converges to under the production/maintenance
    /// flow: `ceil(production * 100 / maintenance_pct)`. There maintenance
    /// cancels production and growth stops — the emergent cap, in place of a
    /// hard constant. Growing from below (as every tile does from
    /// `initial_resources`), a tile lands on exactly this value.
    pub fn resource_equilibrium(&self) -> u32 {
        (self.production * 100).div_ceil(self.maintenance_pct)
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            radius: 5,
            players: 2,
            command_points: 20,
            max_turns: 500,
            initial_army: 20,
            initial_resources: 10,
            production: 2,
            maintenance_pct: 2,
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

impl MoveAmount {
    /// How many armies this order ships out of a stack of `army`
    /// (before the command-point pool clamps it).
    /// The single definition `order_cost` and `step` both use,
    /// so the `Half` rounding can't drift between them.
    fn of(self, army: u32) -> u32 {
        match self {
            MoveAmount::All => army,
            MoveAmount::Half => army / 2,
        }
    }
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
    // Parallel to `hexes` and addressed by `index`; kept crate-visible so the
    // `encoding` module can project the board without a second copy.
    pub(crate) tiles: Vec<Tile>,
    pub(crate) hexes: Vec<Hex>,
    pub(crate) index: HashMap<Hex, usize>,
}

impl GameState {
    pub fn new(config: Config) -> Self {
        assert!(
            (2..=6).contains(&config.players),
            "2 to 6 players supported (one corner each)"
        );
        assert!(config.radius >= 1, "map too small");
        assert!(
            (1..=100).contains(&config.maintenance_pct),
            "maintenance_pct is a percentage in 1..=100"
        );

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

    /// FNV-1a hash of the observable state: the turn counter plus every
    /// tile (owner, army, resources) in canonical map order. Stable across
    /// processes, platforms and Rust versions, so equal hashes from equal
    /// seeds are a meaningful determinism check (see
    /// `engine/tests/invariants.rs`), and RL code can later use it for
    /// transposition tables.
    pub fn state_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        let mut mix = |value: u64| {
            for byte in value.to_le_bytes() {
                hash = (hash ^ byte as u64).wrapping_mul(0x100000001b3);
            }
        };
        mix(self.turn as u64);
        for tile in &self.tiles {
            mix(match tile.owner {
                Some(PlayerId(p)) => p as u64 + 1,
                None => 0,
            });
            mix(tile.army as u64);
            mix(tile.resources as u64);
        }
        hash
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

    /// Command points `order` would spend against the current board —
    /// the same amount `step` charges before clamping to the remaining pool:
    /// a `Move` spends the armies it moves (all, or half rounded down),
    /// a `Recruit` spends the resources it converts.
    /// Orders `step` drops as no-ops
    /// (off-map source or destination, empty source) cost 0.
    /// Ownership is not checked here; callers pass their own tiles' orders.
    pub fn order_cost(&self, order: &Order) -> u32 {
        let Some(&src) = self.index.get(&order.source()) else {
            return 0;
        };
        let tile = &self.tiles[src];
        match *order {
            Order::Recruit { .. } => tile.resources,
            Order::Move { from, dir, amount } => {
                if !self.index.contains_key(&from.neighbor(dir)) {
                    return 0;
                }
                amount.of(tile.army)
            }
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

    /// Resolve one simultaneous turn.
    /// `orders[p]` are player p's orders,
    /// funded in list order from that player's command-point pool
    /// (`Config::command_points`, no accumulation between turns).
    /// Each order costs one CP per army it moves or raises (`order_cost`);
    /// the pool is charged for each,
    /// and once an order can only be paid in part
    /// it moves or raises as many armies as the remaining pool covers
    /// and empties it, dropping the rest.
    /// Illegal orders
    /// (off-map, not owned by the player,
    /// a second order from the same source tile)
    /// are dropped silently without charge.
    ///
    /// Each player's orders apply to their own tiles only, so the player
    /// processing order is irrelevant: departures and recruits happen
    /// "at once", then everything lands and fights, then resources flow
    /// (production minus maintenance).
    /// Combat is mutual attack with no defense: only armies commanded to
    /// move strike, so a stationary garrison deals no damage and a winning
    /// attacker keeps its full force (see the combat step below).
    /// One order per tile per turn also means recruited armies join the
    /// garrison immediately but cannot move until the next turn.
    pub fn step(&mut self, orders: &[Vec<Order>]) -> Outcome {
        assert_eq!(orders.len(), self.config.players as usize);

        let mut acted: Vec<bool> = vec![false; self.tiles.len()];
        // arrivals[tile] = per-player armies landing there this turn
        let mut arrivals: HashMap<usize, HashMap<PlayerId, u64>> = HashMap::new();

        for (p, player_orders) in orders.iter().enumerate() {
            let player = PlayerId(p as u8);
            let mut budget = self.config.command_points;
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
                        // One CP per army raised;
                        // a pool that can't cover the whole tile
                        // converts as many resources as it can.
                        let converting = tile.resources.min(budget);
                        if converting == 0 {
                            continue;
                        }
                        tile.army += converting;
                        tile.resources -= converting;
                        budget -= converting;
                    }
                    Order::Move { from, dir, amount } => {
                        let Some(&dst) = self.index.get(&from.neighbor(dir)) else {
                            continue;
                        };
                        // One CP per army moved; the pool caps how many go.
                        let moving = amount.of(self.tiles[src].army).min(budget);
                        if moving == 0 {
                            continue;
                        }
                        self.tiles[src].army -= moving;
                        *arrivals.entry(dst).or_default().entry(player).or_default() +=
                            moving as u64;
                        budget -= moving;
                    }
                }
                acted[src] = true;
            }
        }

        for (dst, parties) in arrivals {
            let tile = &mut self.tiles[dst];
            // Neutral tiles never hold armies (ownership never reverts), so
            // there is no "neutral garrison" party to account for.
            debug_assert!(tile.owner.is_some() || tile.army == 0);

            // There is no defense, only mutual attack: a stationary garrison
            // deals no damage — to strike, an army must be ordered to move.
            // Track each party as (player, presence, attack): presence
            // (arrivals, plus the owner's garrison) decides who holds the
            // tile; attack (arrivals alone) is what costs the others armies.
            let owner = tile.owner;
            let mut forces: Vec<(PlayerId, u64, u64)> =
                parties.into_iter().map(|(p, arr)| (p, arr, arr)).collect();
            // Fold the owner's garrison into its presence, adding the owner as
            // a pure-garrison party when it commanded no arrivals of its own.
            let garrison = tile.army as u64;
            if let Some(o) = owner.filter(|_| garrison > 0) {
                match forces.iter_mut().find(|f| f.0 == o) {
                    Some(f) => f.1 += garrison,
                    None => forces.push((o, garrison, 0)),
                }
            }

            if forces.len() == 1 {
                let (player, present, _) = forces[0];
                tile.owner = Some(player);
                tile.army = present.min(u32::MAX as u64) as u32;
                continue;
            }

            // Deterministic single-step combat: the largest presence holds the
            // tile, paying only the combined attack of everyone else — a
            // garrison strikes for nothing, so overrunning it is free. The
            // owner wins ties, so an attacker that merely matches a garrison
            // annihilates against it rather than taking the tile.
            forces
                .sort_by_key(|&(p, present, _)| (std::cmp::Reverse(present), Some(p) != owner, p));

            let (winner, winner_present, _) = forces[0];
            let losses: u64 = forces[1..].iter().map(|&(_, _, attack)| attack).sum();

            if winner_present <= losses {
                // Mutual annihilation (covers a tie for the largest presence,
                // since the runner-up's attack alone then matches the winner):
                // the tile keeps its owner but is left undefended.
                tile.army = 0;
                continue;
            }
            tile.owner = Some(winner);
            tile.army = (winner_present - losses).min(u32::MAX as u64) as u32;
        }

        // Flat production minus stockpile-scaled maintenance; converges up to
        // `Config::resource_equilibrium`. `maintenance_pct <= 100` keeps the
        // maintenance from exceeding the stockpile, so the subtraction is safe.
        for tile in &mut self.tiles {
            let maintenance =
                (tile.resources as u64 * self.config.maintenance_pct as u64 / 100) as u32;
            tile.resources = (tile.resources - maintenance).saturating_add(self.config.production);
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
            // Ample CP so mechanic tests are never truncated by the pool;
            // the `command_points_*` tests exercise the pool itself.
            command_points: u32::MAX,
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
    fn command_points_pool_is_shared_across_tiles() {
        // A pool of exactly the starting army:
        // moving that whole stack one tile spends it all,
        // leaving nothing for a later order elsewhere.
        let initial = Config::default().initial_army;
        let mut s = GameState::new(Config {
            command_points: initial,
            ..small_config()
        });
        let start = start_of(&s, PlayerId(0));
        let dir = some_in_map_direction(&s, start);
        // Give player 0 a second tile so two distinct sources exist.
        // The move from `start` lands here,
        // so its post-turn army also proves the recruit never fired.
        let second = start.neighbor(dir);
        s.set_tile(
            second,
            Tile {
                owner: Some(PlayerId(0)),
                army: 0,
                resources: 40,
            },
        );

        // [Move all of start, Recruit at second]:
        // the move drains the pool, so the recruit gets no CP and is dropped.
        s.step(&[
            vec![move_all(start, dir), Order::Recruit { at: second }],
            vec![],
        ]);

        assert_eq!(
            s.tile(start).unwrap().army,
            0,
            "listed-first move must spend the pool"
        );
        assert_eq!(
            s.tile(second).unwrap().army,
            initial,
            "recruit must be dropped once the pool is empty (only the moved army is here)"
        );
    }

    #[test]
    fn move_is_capped_by_command_points() {
        // Pool smaller than the stack:
        // only pool-many armies move, the rest stay put.
        let mut s = GameState::new(Config {
            command_points: 5,
            ..small_config()
        });
        let start = start_of(&s, PlayerId(0));
        let dir = some_in_map_direction(&s, start);
        let dest = start.neighbor(dir);
        let initial = s.config.initial_army;

        s.step(&[vec![move_all(start, dir)], vec![]]);

        assert_eq!(
            s.tile(start).unwrap().army,
            initial - 5,
            "stack keeps the unpaid armies"
        );
        assert_eq!(s.tile(dest).unwrap().army, 5, "only pool-many armies move");
    }

    #[test]
    fn recruit_is_capped_by_command_points() {
        // One CP per army raised: a pool of 4 converts only 4 resources.
        let mut s = GameState::new(Config {
            command_points: 4,
            ..small_config()
        });
        let start = start_of(&s, PlayerId(0));
        s.set_tile(
            start,
            Tile {
                owner: Some(PlayerId(0)),
                army: 0,
                resources: 30,
            },
        );

        s.step(&[vec![Order::Recruit { at: start }], vec![]]);

        assert_eq!(
            s.tile(start).unwrap().army,
            4,
            "only pool-many resources become armies"
        );
    }

    #[test]
    fn equal_attack_annihilates_both_sides() {
        let mut s = state();
        let initial = s.config.initial_army;
        let (p0_start, attacker_hex, back) = stage_attack(&mut s, initial);

        s.step(&[vec![], vec![move_all(attacker_hex, back)]]);

        // The attacker only matches the garrison, so neither holds the tile:
        // it stays the owner's but is left undefended.
        let defended = s.tile(p0_start).unwrap();
        assert_eq!(defended.owner, Some(PlayerId(0)));
        assert_eq!(defended.army, 0);
    }

    #[test]
    fn understrength_attack_leaves_the_garrison_standing() {
        let mut s = state();
        let initial = s.config.initial_army;
        // Attacker musters one fewer army than the garrison it charges.
        let (p0_start, attacker_hex, back) = stage_attack(&mut s, initial - 1);

        s.step(&[vec![], vec![move_all(attacker_hex, back)]]);

        // The garrison took the attacker's fire and holds with what is left
        // (initial - (initial - 1) = 1).
        let defended = s.tile(p0_start).unwrap();
        assert_eq!(defended.owner, Some(PlayerId(0)));
        assert_eq!(defended.army, 1);
    }

    #[test]
    fn winning_attacker_keeps_its_full_force() {
        let mut s = state();
        let initial = s.config.initial_army;
        // One army over the garrison is enough to take the tile.
        let attackers = initial + 1;
        let (p0_start, attacker_hex, back) = stage_attack(&mut s, attackers);

        s.step(&[vec![], vec![move_all(attacker_hex, back)]]);

        // The garrison dealt no damage, so the attacker keeps every army it
        // brought — the whole point of "no defense, only mutual attack".
        let taken = s.tile(p0_start).unwrap();
        assert_eq!(taken.owner, Some(PlayerId(1)));
        assert_eq!(taken.army, attackers);
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

        // The garrison strikes for nothing, so a u32::MAX attacker survives at
        // full strength — the count is clamped to the tile's u32, not wrapped.
        let taken = s.tile(p0_start).unwrap();
        assert_eq!(taken.owner, Some(PlayerId(1)));
        assert_eq!(taken.army, u32::MAX);
    }

    #[test]
    fn resources_converge_to_equilibrium() {
        let mut s = state();
        // Far more turns than the ~100 a tile needs to climb from
        // `initial_resources` to the equilibrium and stop there.
        for _ in 0..s.config.max_turns {
            s.step(&[vec![], vec![]]);
        }
        let eq = s.config.resource_equilibrium();
        assert!(s.iter_tiles().all(|(_, t)| t.resources == eq));
    }
}
