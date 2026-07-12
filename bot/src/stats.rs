//! Match records and series-level game-health metrics — the definitions
//! behind `DESIGN.md` "How we know it works as a game". `MatchRecord` is
//! the minimal per-game trace the metrics need; `SeriesStats` aggregates
//! records into the numbers the CLI prints and `bot/tests/health.rs`
//! asserts.

use overthrow_engine::{GameState, Outcome, PlayerId};

/// One completed match: its outcome plus the per-turn leader trace.
/// `leaders[t]` is the strict tile-count leader after `t` turns have
/// resolved (`None` = tied), so `leaders[0]` describes the initial state
/// and `leaders[turns]` the final one.
#[derive(Clone, Debug)]
pub struct MatchRecord {
    pub outcome: Outcome,
    pub turns: u32,
    pub max_turns: u32,
    pub leaders: Vec<Option<PlayerId>>,
}

/// The strict tile-count leader of the current state, `None` on a tie.
pub fn strict_leader(state: &GameState) -> Option<PlayerId> {
    let counts: Vec<usize> = (0..state.config.players)
        .map(|p| state.tile_count(PlayerId(p)))
        .collect();
    let best = *counts.iter().max().unwrap();
    let mut leaders = counts.iter().enumerate().filter(|&(_, &c)| c == best);
    let (player, _) = leaders.next().unwrap();
    match leaders.next() {
        Some(_) => None,
        None => Some(PlayerId(player as u8)),
    }
}

impl MatchRecord {
    pub fn winner(&self) -> Option<PlayerId> {
        match self.outcome {
            Outcome::Winner(p) => Some(p),
            _ => None,
        }
    }

    /// Whether the game ended by eliminating every other player before
    /// the turn limit, as opposed to being adjudicated (or drawn) at it.
    /// A kill landing exactly on the limit is indistinguishable from
    /// adjudication in the record and counts as a turn-limit ending.
    pub fn by_elimination(&self) -> bool {
        self.winner().is_some() && self.turns < self.max_turns
    }

    /// The strict tile-count leader at the quarter mark of the game
    /// (`None` if tied there). The reference point for `comeback`.
    pub fn early_leader(&self) -> Option<PlayerId> {
        let quarter = (self.turns as usize / 4).max(1).min(self.leaders.len() - 1);
        self.leaders[quarter]
    }

    /// Whether the eventual winner was *behind* at the quarter mark —
    /// the anti-snowball design goal made measurable. `None` when the
    /// game had no winner or no strict early leader.
    pub fn comeback(&self) -> Option<bool> {
        Some(self.winner()? != self.early_leader()?)
    }
}

/// Aggregated game-health metrics over a series of matches.
#[derive(Clone, Debug, Default)]
pub struct SeriesStats {
    pub games: u32,
    /// Wins per player id.
    pub wins: Vec<u32>,
    pub draws: u32,
    /// Games won by eliminating every opponent (see
    /// `MatchRecord::by_elimination`).
    pub eliminations: u32,
    /// Games that ran to the turn limit (adjudicated wins and draws).
    pub turn_limit_endings: u32,
    /// Decided games where the early leader lost (see
    /// `MatchRecord::comeback`).
    pub comebacks: u32,
    /// Decided games that had a strict early leader — the denominator
    /// for the comeback rate.
    pub comeback_eligible: u32,
    pub total_turns: u64,
}

impl SeriesStats {
    pub fn record(&mut self, record: &MatchRecord) {
        self.games += 1;
        self.total_turns += record.turns as u64;
        match record.winner() {
            Some(PlayerId(p)) => {
                if self.wins.len() <= p as usize {
                    self.wins.resize(p as usize + 1, 0);
                }
                self.wins[p as usize] += 1;
            }
            None => self.draws += 1,
        }
        if record.by_elimination() {
            self.eliminations += 1;
        } else if record.turns >= record.max_turns {
            self.turn_limit_endings += 1;
        }
        if let Some(comeback) = record.comeback() {
            self.comeback_eligible += 1;
            if comeback {
                self.comebacks += 1;
            }
        }
    }

    pub fn wins_of(&self, player: PlayerId) -> u32 {
        self.wins.get(player.0 as usize).copied().unwrap_or(0)
    }

    pub fn avg_turns(&self) -> u64 {
        self.total_turns / self.games.max(1) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(outcome: Outcome, leaders: Vec<Option<PlayerId>>) -> MatchRecord {
        MatchRecord {
            outcome,
            turns: leaders.len() as u32 - 1,
            max_turns: 100,
            leaders,
        }
    }

    #[test]
    fn comeback_is_winner_behind_at_quarter_mark() {
        let p0 = Some(PlayerId(0));
        let p1 = Some(PlayerId(1));
        // 8 turns; quarter mark = turn 2, where p1 leads.
        let leaders = vec![None, p0, p1, p1, p1, p0, p0, p0, p0];
        assert_eq!(
            record(Outcome::Winner(PlayerId(0)), leaders.clone()).comeback(),
            Some(true)
        );
        assert_eq!(
            record(Outcome::Winner(PlayerId(1)), leaders.clone()).comeback(),
            Some(false)
        );
        assert_eq!(record(Outcome::Draw, leaders).comeback(), None);
        // Tied at the quarter mark: no reference point, not eligible.
        let tied = vec![None, None, None, None, None, p0, p0, p0, p0];
        assert_eq!(record(Outcome::Winner(PlayerId(0)), tied).comeback(), None);
    }

    #[test]
    fn series_stats_classify_endings() {
        let p0 = Some(PlayerId(0));
        let p1 = Some(PlayerId(1));
        let mut stats = SeriesStats::default();
        // Elimination win with a comeback (p1 led the quarter mark).
        stats.record(&record(
            Outcome::Winner(PlayerId(0)),
            vec![None, p1, p1, p1, p0, p0, p0, p0, p0],
        ));
        // Turn-limit draw.
        stats.record(&MatchRecord {
            outcome: Outcome::Draw,
            turns: 100,
            max_turns: 100,
            leaders: vec![None; 101],
        });
        assert_eq!(stats.games, 2);
        assert_eq!(stats.wins_of(PlayerId(0)), 1);
        assert_eq!(stats.draws, 1);
        assert_eq!(stats.eliminations, 1);
        assert_eq!(stats.turn_limit_endings, 1);
        assert_eq!((stats.comebacks, stats.comeback_eligible), (1, 1));
    }
}
