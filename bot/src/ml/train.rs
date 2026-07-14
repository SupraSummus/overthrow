//! Self-contained REINFORCE trainer for the [`Policy`], the "learn" half of
//! the vertical slice. It plays real engine games with the policy in one
//! seat and a scripted opponent in the other, then nudges the weights toward
//! whichever sampled actions led to owning more of the board.
//!
//! Deliberately the simplest thing that learns: REINFORCE with a dense
//! per-turn reward (the change in the learner's tile lead), discounted
//! reward-to-go so each decision is credited for what followed it, a
//! batch-mean baseline for variance reduction, and Adam. Reward-to-go is
//! what makes the signal usable at all here — a single terminal reward split
//! across the hundreds of per-tile decisions in a game barely moves the
//! policy. It is enough to beat `random` and to prove the encoding and
//! action space are learnable end to end; the heavier CNN/GNN + PPO path in
//! `DESIGN.md` can reuse the same `engine::encoding` contract when richer
//! play is wanted.

use overthrow_engine::rng::Rng;
use overthrow_engine::{Config, GameState, Order, Outcome, PlayerId};

use crate::ml::policy::{Adam, Decision, Grads, Policy};
use crate::{make_bot, Bot};

/// Reward-to-go discount: how much a decision is credited for territory
/// swings several turns later. Below 1 so near-term consequences dominate.
const GAMMA: f32 = 0.99;

/// Added to the final turn's reward for an outright win (subtracted for a
/// loss), so finishing the game beats merely leading on tiles at the cap.
const WIN_BONUS: f32 = 1.0;

/// Knobs for a training run.
pub struct TrainParams {
    pub radius: i32,
    pub max_turns: u32,
    pub opponent: String,
    pub updates: u32,
    pub batch: u32,
    pub lr: f32,
    pub seed: u64,
    pub eval_every: u32,
    pub eval_games: u32,
}

impl Default for TrainParams {
    fn default() -> Self {
        TrainParams {
            radius: 3,
            max_turns: 100,
            opponent: "random".into(),
            updates: 3000,
            batch: 32,
            lr: 0.02,
            seed: 1,
            eval_every: 200,
            eval_games: 200,
        }
    }
}

impl TrainParams {
    fn config(&self) -> Config {
        Config {
            radius: self.radius,
            players: 2,
            max_turns: self.max_turns,
            ..Config::default()
        }
    }
}

/// Train a fresh policy and return the best one seen on the periodic
/// evaluation (win rate against the opponent). Progress is reported through
/// `report`, so a CLI can print it without this module owning stdout.
pub fn train(params: &TrainParams, report: &mut dyn FnMut(String)) -> Policy {
    let config = params.config();
    let mut rng = Rng::new(params.seed ^ 0x5eed);
    let mut policy = Policy::init(&mut rng);
    let mut opt = Adam::new();

    let mut best = policy.clone();
    let mut best_winrate = eval(
        &best,
        &config,
        &params.opponent,
        params.eval_games,
        params.seed,
    );
    report(format!("update 0: win rate {best_winrate:.3} (initial)"));

    for update in 1..=params.updates {
        let mut grads = Grads::zeros();
        // Every sampled decision paired with its discounted reward-to-go.
        let mut samples: Vec<(Decision, f32)> = Vec::new();
        let mut mean_return = 0.0f32;
        for ep in 0..params.batch {
            // Alternate seats so the policy learns from both corners; the
            // opponent gets a fresh seed per game.
            let learner_seat = (ep % 2) as usize;
            let ep_seed = params
                .seed
                .wrapping_mul(0x9E37)
                .wrapping_add(update as u64 * params.batch as u64 + ep as u64);
            let mut opp = make_bot(&params.opponent, ep_seed).unwrap_or_else(|| {
                panic!("unknown opponent bot: {}", params.opponent);
            });
            let (steps, _) = play(&policy, &config, learner_seat, opp.as_mut(), Some(&mut rng));
            // Discounted reward-to-go: credit each turn's decisions with the
            // (discounted) sum of rewards from that turn onward.
            let mut g = 0.0;
            for step in steps.into_iter().rev() {
                g = step.reward + GAMMA * g;
                mean_return += g * step.decisions.len() as f32;
                for d in step.decisions {
                    samples.push((d, g));
                }
            }
        }

        if samples.is_empty() {
            continue;
        }
        // Batch-mean baseline: advantage is a decision's reward-to-go minus
        // the batch average, which centres the gradient without bias.
        mean_return /= samples.len() as f32;
        for (d, g) in &samples {
            policy.backward_into(d, g - mean_return, &mut grads);
        }
        grads.scale(1.0 / samples.len() as f32);
        policy.adam_step(&grads, &mut opt, params.lr);

        if params.eval_every > 0 && update % params.eval_every == 0 {
            let winrate = eval(
                &policy,
                &config,
                &params.opponent,
                params.eval_games,
                params.seed,
            );
            report(format!(
                "update {update}: win rate {winrate:.3} vs {} (mean reward-to-go {mean_return:+.3})",
                params.opponent,
            ));
            if winrate >= best_winrate {
                best_winrate = winrate;
                best = policy.clone();
            }
        }
    }
    report(format!("best win rate {best_winrate:.3}"));
    best
}

/// One turn's worth of the learner's sampled decisions and the reward that
/// followed resolving the turn.
struct Step {
    decisions: Vec<Decision>,
    reward: f32,
}

/// Play one full game: the policy drives `learner_seat`, `opp` drives the
/// other seat. With `rng = Some`, the policy samples and each turn's
/// [`Decision`]s are recorded (for training); with `None` it plays the
/// deterministic argmax and no decisions are recorded (for evaluation).
/// Returns the per-turn steps and the terminal state.
///
/// The per-turn reward is the change in the learner's *tile lead* (its tiles
/// minus its opponents') caused by that turn, plus [`WIN_BONUS`] on the turn
/// it wins (minus it on the turn it loses). Summed over a game this
/// telescopes to the final lead, so it is the same objective as "own more of
/// the board" but delivered turn by turn, where it can be credited.
fn play(
    policy: &Policy,
    config: &Config,
    learner_seat: usize,
    opp: &mut dyn Bot,
    mut rng: Option<&mut Rng>,
) -> (Vec<Step>, GameState) {
    let me = PlayerId(learner_seat as u8);
    let mut state = GameState::new(config.clone());
    let mut steps = Vec::new();
    loop {
        let lead_before = lead(&state, me);
        let mut orders: Vec<Vec<Order>> = vec![Vec::new(); config.players as usize];
        let mut decisions = Vec::new();
        for (p, slot) in orders.iter_mut().enumerate() {
            let seat = PlayerId(p as u8);
            if p == learner_seat {
                let (funded, ds) = policy.orders(&state, seat, rng.as_deref_mut());
                decisions = ds;
                *slot = funded;
            } else {
                *slot = opp.orders(&state, seat);
            }
        }
        let outcome = state.step(&orders);
        let mut reward = (lead(&state, me) - lead_before) as f32;
        reward += match outcome {
            Outcome::Winner(p) if p == me => WIN_BONUS,
            Outcome::Winner(_) => -WIN_BONUS,
            _ => 0.0,
        };
        steps.push(Step { decisions, reward });
        if outcome != Outcome::Ongoing {
            return (steps, state);
        }
    }
}

/// The learner's tile lead: its tile count minus its opponents' combined.
fn lead(state: &GameState, me: PlayerId) -> i64 {
    let mut lead = 0;
    for p in 0..state.config.players {
        let count = state.tile_count(PlayerId(p)) as i64;
        lead += if PlayerId(p) == me { count } else { -count };
    }
    lead
}

/// Win rate of the greedily-played policy against `opponent` over `games`
/// games, alternating seats. Draws and losses count as non-wins.
pub fn eval(policy: &Policy, config: &Config, opponent: &str, games: u32, seed: u64) -> f32 {
    let mut wins = 0u32;
    for i in 0..games {
        let learner_seat = (i % 2) as usize;
        let mut opp = make_bot(opponent, seed.wrapping_add(i as u64 * 131 + 7))
            .unwrap_or_else(|| panic!("unknown opponent bot: {opponent}"));
        let (_, state) = play(policy, config, learner_seat, opp.as_mut(), None);
        if state.outcome() == Outcome::Winner(PlayerId(learner_seat as u8)) {
            wins += 1;
        }
    }
    wins as f32 / games.max(1) as f32
}
