//! The learned policy: a tiny two-layer perceptron applied independently to
//! every owned tile over its one-hop hex patch (the tile plus its six
//! neighbours). One shared set of weights slides over the board like a
//! 1x1-neighbourhood graph convolution, so the parameter count does not grow
//! with the map and a policy trained at one radius transfers to another.
//!
//! Per tile the head emits a distribution over that tile's
//! [`overthrow_engine::ACTIONS_PER_TILE`] orders plus one extra "pass here"
//! action, masked to the orders the engine would actually accept. The
//! forward path is all this crate needs to *play*; the gradient path
//! ([`Grads`], [`Adam`], [`Policy::backward_into`]) is what
//! [`crate::ml::train`] needs to *learn*, and it lives here so it can touch
//! the weight buffers directly.

use std::fmt::Write as _;

use overthrow_engine::rng::Rng;
use overthrow_engine::{Direction, GameState, Hex, Order, PlayerId, ACTIONS_PER_TILE, NUM_PLANES};

use crate::take_budget;

/// Hidden units in the perceptron. Small on purpose: the whole point of the
/// vertical slice is a policy that trains in seconds and runs in
/// microseconds, embeddable in the app.
pub const HIDDEN: usize = 16;

/// Tiles in a one-hop patch: the centre tile and its six neighbours.
const PATCH_TILES: usize = 1 + Direction::ALL.len();

/// Input width: every patch tile contributes [`NUM_PLANES`] features.
pub const INPUT: usize = PATCH_TILES * NUM_PLANES;

/// Output width: one logit per tile-rooted order, plus a final "pass"
/// logit that lets the policy decline to act on a tile (spending no command
/// points there).
pub const HEAD: usize = ACTIONS_PER_TILE + 1;

/// Index of the "pass" action within the head.
const PASS: usize = HEAD - 1;

/// The four parameter blocks, in one fixed order that everything — layout,
/// serialization, the optimizer, the gradient buffers — shares: the first
/// layer's weights and bias, then the second's. `w1`/`b1` map an [`INPUT`]
/// vector to [`HIDDEN`] units through `tanh`; `w2`/`b2` map those to [`HEAD`]
/// logits. Weight matrices are row-major (`w1[k * INPUT + i]`,
/// `w2[j * HIDDEN + k]`).
const SIZES: [usize; 4] = [HIDDEN * INPUT, HIDDEN, HEAD * HIDDEN, HEAD];
const W1: usize = 0;
const B1: usize = 1;
const W2: usize = 2;
const B2: usize = 3;

/// Four zeroed blocks sized by [`SIZES`] — a fresh gradient accumulator or
/// optimizer moment.
fn zero_blocks() -> [Vec<f32>; 4] {
    SIZES.map(|n| vec![0.0; n])
}

/// A frozen record of one sampled decision, enough to replay the forward
/// pass and backpropagate without storing activations: the patch input, the
/// legality mask that shaped the distribution, and the action taken.
#[derive(Clone)]
pub struct Decision {
    pub x: [f32; INPUT],
    pub allowed: [bool; HEAD],
    pub action: usize,
}

/// The policy weights, as the four [`SIZES`] blocks.
#[derive(Clone)]
pub struct Policy {
    buf: [Vec<f32>; 4],
}

impl Policy {
    /// Small random weights (scaled by fan-in), biases zero — the starting
    /// point for training.
    pub fn init(rng: &mut Rng) -> Self {
        let scale1 = (1.0 / INPUT as f32).sqrt();
        let scale2 = (1.0 / HIDDEN as f32).sqrt();
        let buf = std::array::from_fn(|block| {
            let scale = match block {
                W1 => scale1,
                W2 => scale2,
                _ => return vec![0.0; SIZES[block]], // biases
            };
            (0..SIZES[block]).map(|_| gaussian(rng) * scale).collect()
        });
        Policy { buf }
    }

    fn w1(&self) -> &[f32] {
        &self.buf[W1]
    }
    fn b1(&self) -> &[f32] {
        &self.buf[B1]
    }
    fn w2(&self) -> &[f32] {
        &self.buf[W2]
    }
    fn b2(&self) -> &[f32] {
        &self.buf[B2]
    }

    /// The centre-plus-neighbours feature patch feeding one tile's head,
    /// in `[self, dir0, dir1, ...]` order. Off-map neighbours read as zeros
    /// (same as neutral empty tiles on the army planes — see
    /// `GameState::tile_features`).
    fn patch(state: &GameState, hex: Hex, me: PlayerId) -> [f32; INPUT] {
        let mut x = [0.0; INPUT];
        let mut write = |slot: usize, f: [f32; NUM_PLANES]| {
            x[slot * NUM_PLANES..(slot + 1) * NUM_PLANES].copy_from_slice(&f);
        };
        write(0, state.tile_features(hex, me));
        for (i, (_, n)) in hex.neighbors().enumerate() {
            write(1 + i, state.tile_features(n, me));
        }
        x
    }

    /// Forward pass for one patch: hidden activations and raw logits.
    fn forward(&self, x: &[f32; INPUT]) -> ([f32; HIDDEN], [f32; HEAD]) {
        let mut h = [0.0f32; HIDDEN];
        for (k, hk) in h.iter_mut().enumerate() {
            *hk = (self.b1()[k] + dot(&self.w1()[k * INPUT..(k + 1) * INPUT], x)).tanh();
        }
        let mut logits = [0.0f32; HEAD];
        for (j, lj) in logits.iter_mut().enumerate() {
            *lj = self.b2()[j] + dot(&self.w2()[j * HIDDEN..(j + 1) * HIDDEN], &h);
        }
        (h, logits)
    }

    /// This player's orders for the turn, funded from the command-point pool
    /// most-confident-first, plus (when sampling) the [`Decision`]s to learn
    /// from. With `rng = Some` the per-tile action is sampled (training); with
    /// `None` it is the argmax (deterministic play). This is the one path from
    /// a policy to a legal, funded turn — `MlBot` and the trainer both use it.
    pub fn orders(
        &self,
        state: &GameState,
        me: PlayerId,
        rng: Option<&mut Rng>,
    ) -> (Vec<Order>, Vec<Decision>) {
        let (mut scored, decisions) = self.select(state, me, rng);
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        let orders = take_budget(state, scored.into_iter().map(|(_, order)| order));
        (orders, decisions)
    }

    /// Choose one action per owned tile — sampling from the masked
    /// distribution when `rng` is `Some` (training) or taking the argmax when
    /// `None` (deterministic play) — and turn the non-pass choices into
    /// orders paired with the probability the policy gave them (so `orders`
    /// can fund the most-wanted first), plus, when sampling, the
    /// [`Decision`]s to learn from.
    ///
    /// The per-tile legality mask is sliced out of `GameState::legal_action_mask`,
    /// so the policy can only ever pick orders the engine would accept.
    fn select(
        &self,
        state: &GameState,
        me: PlayerId,
        mut rng: Option<&mut Rng>,
    ) -> (Vec<(f32, Order)>, Vec<Decision>) {
        let mask = state.legal_action_mask(me);
        let mut orders = Vec::new();
        let mut decisions = Vec::new();
        for (hex, tile) in state.iter_tiles() {
            if tile.owner != Some(me) {
                continue;
            }
            let base = state.tile_index(hex).unwrap() * ACTIONS_PER_TILE;
            let mut allowed = [false; HEAD];
            allowed[..ACTIONS_PER_TILE].copy_from_slice(&mask[base..base + ACTIONS_PER_TILE]);
            if !allowed[..ACTIONS_PER_TILE].iter().any(|&a| a) {
                continue; // Only pass is possible; nothing to decide.
            }
            allowed[PASS] = true;

            let x = Self::patch(state, hex, me);
            let (_, logits) = self.forward(&x);
            let p = softmax_masked(&logits, &allowed);
            let action = match rng.as_deref_mut() {
                Some(rng) => sample(&p, rng),
                None => argmax(&p),
            };
            if rng.is_some() {
                decisions.push(Decision { x, allowed, action });
            }
            if action != PASS {
                orders.push((p[action], state.action_to_order(base + action).unwrap()));
            }
        }
        (orders, decisions)
    }

    /// Accumulate the REINFORCE gradient of one decision into `grads`,
    /// weighted by `coef` (the advantage: return minus baseline). Replays
    /// the forward pass, then backpropagates
    /// `d(-coef * log p[action]) / d(params)` through the head and the
    /// `tanh` layer.
    pub fn backward_into(&self, d: &Decision, coef: f32, grads: &mut Grads) {
        let (h, logits) = self.forward(&d.x);
        let p = softmax_masked(&logits, &d.allowed);
        let g = &mut grads.buf;

        // dLoss/dlogit_j = coef * (p_j - [j == action]); masked-out logits
        // are constants (p_j = 0) and get no gradient.
        let mut dh = [0.0f32; HIDDEN];
        for (j, &pj) in p.iter().enumerate() {
            if !d.allowed[j] {
                continue;
            }
            let dj = coef * (pj - if j == d.action { 1.0 } else { 0.0 });
            g[B2][j] += dj;
            let row = j * HIDDEN;
            for k in 0..HIDDEN {
                g[W2][row + k] += dj * h[k];
                dh[k] += dj * self.w2()[row + k];
            }
        }
        for k in 0..HIDDEN {
            let dz = dh[k] * (1.0 - h[k] * h[k]); // tanh'
            g[B1][k] += dz;
            let row = k * INPUT;
            for i in 0..INPUT {
                g[W1][row + i] += dz * d.x[i];
            }
        }
    }

    /// One Adam step: update every weight from `grads` and the optimizer
    /// moments in `opt`.
    pub fn adam_step(&mut self, grads: &Grads, opt: &mut Adam, lr: f32) {
        opt.t += 1;
        let t = opt.t;
        for (((w, g), m), v) in self
            .buf
            .iter_mut()
            .zip(&grads.buf)
            .zip(&mut opt.m)
            .zip(&mut opt.v)
        {
            adam_vec(w, g, m, v, t, lr);
        }
    }

    /// Serialize to a self-describing whitespace-separated text blob:
    /// a magic tag, a format version, the three layer widths (so a stale
    /// checkpoint is rejected rather than silently misread), then every
    /// weight in [`SIZES`] order. Rust's `f32` formatting is
    /// shortest-round-trip, so parsing recovers the exact bits.
    pub fn serialize(&self) -> String {
        let mut s = String::new();
        let _ = write!(s, "overthrow-policy 1 {HIDDEN} {INPUT} {HEAD}");
        for value in self.buf.iter().flatten() {
            let _ = write!(s, " {value}");
        }
        s.push('\n');
        s
    }

    /// Parse a checkpoint written by `serialize`, rejecting a blob whose
    /// widths or weight count don't match this build.
    pub fn deserialize(text: &str) -> Result<Self, String> {
        let mut tok = text.split_whitespace();
        let mut next = |what: &str| tok.next().ok_or_else(|| format!("missing {what}"));
        if next("magic")? != "overthrow-policy" {
            return Err("not an overthrow policy checkpoint".into());
        }
        if next("version")? != "1" {
            return Err("unsupported checkpoint version".into());
        }
        let dim = |t: &str| t.parse::<usize>().map_err(|_| format!("bad dim {t:?}"));
        let shape = [
            next("hidden").and_then(dim)?,
            next("input").and_then(dim)?,
            next("head").and_then(dim)?,
        ];
        if shape != [HIDDEN, INPUT, HEAD] {
            let [h, i, o] = shape;
            return Err(format!(
                "checkpoint shape {h}x{i}x{o} != build {HIDDEN}x{INPUT}x{HEAD}"
            ));
        }
        let weights = tok.map(parse).collect::<Result<Vec<f32>, _>>()?;
        if weights.len() != SIZES.iter().sum() {
            return Err("checkpoint weight count does not match its shape".into());
        }
        let mut it = weights.into_iter();
        Ok(Policy {
            buf: SIZES.map(|n| it.by_ref().take(n).collect()),
        })
    }

    /// The REINFORCE loss of one decision, `-coef * log p[action]` — the
    /// scalar `backward_into` differentiates. Test-only reference.
    #[cfg(test)]
    fn loss(&self, d: &Decision, coef: f32) -> f32 {
        let (_, logits) = self.forward(&d.x);
        let p = softmax_masked(&logits, &d.allowed);
        -coef * p[d.action].ln()
    }
}

/// Gradient accumulator, the same four [`SIZES`] blocks as a [`Policy`].
pub struct Grads {
    buf: [Vec<f32>; 4],
}

impl Grads {
    pub fn zeros() -> Self {
        Grads { buf: zero_blocks() }
    }

    /// Scale every accumulated gradient (used to average over a batch).
    pub fn scale(&mut self, factor: f32) {
        for value in self.buf.iter_mut().flatten() {
            *value *= factor;
        }
    }
}

/// Adam optimizer moments (first and second) for every weight.
pub struct Adam {
    t: u32,
    m: [Vec<f32>; 4],
    v: [Vec<f32>; 4],
}

impl Adam {
    pub fn new() -> Self {
        Adam {
            t: 0,
            m: zero_blocks(),
            v: zero_blocks(),
        }
    }
}

impl Default for Adam {
    fn default() -> Self {
        Self::new()
    }
}

const BETA1: f32 = 0.9;
const BETA2: f32 = 0.999;
const EPS: f32 = 1e-8;

fn adam_vec(w: &mut [f32], g: &[f32], m: &mut [f32], v: &mut [f32], t: u32, lr: f32) {
    let bc1 = 1.0 - BETA1.powi(t as i32);
    let bc2 = 1.0 - BETA2.powi(t as i32);
    for i in 0..w.len() {
        m[i] = BETA1 * m[i] + (1.0 - BETA1) * g[i];
        v[i] = BETA2 * v[i] + (1.0 - BETA2) * g[i] * g[i];
        w[i] -= lr * (m[i] / bc1) / ((v[i] / bc2).sqrt() + EPS);
    }
}

fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn parse(tok: &str) -> Result<f32, String> {
    tok.parse().map_err(|_| format!("bad float {tok:?}"))
}

/// Softmax over the allowed logits; disallowed entries get probability zero.
/// `allowed` always has at least the pass action set, so the sum is positive.
fn softmax_masked(logits: &[f32; HEAD], allowed: &[bool; HEAD]) -> [f32; HEAD] {
    let max = (0..HEAD)
        .filter(|&j| allowed[j])
        .map(|j| logits[j])
        .fold(f32::NEG_INFINITY, f32::max);
    let mut p = [0.0f32; HEAD];
    let mut sum = 0.0;
    for j in 0..HEAD {
        if allowed[j] {
            p[j] = (logits[j] - max).exp();
            sum += p[j];
        }
    }
    for value in &mut p {
        *value /= sum;
    }
    p
}

fn argmax(p: &[f32; HEAD]) -> usize {
    (0..HEAD).max_by(|&a, &b| p[a].total_cmp(&p[b])).unwrap()
}

fn sample(p: &[f32; HEAD], rng: &mut Rng) -> usize {
    let mut u = uniform(rng);
    for (j, &pj) in p.iter().enumerate() {
        u -= pj;
        if u <= 0.0 {
            return j;
        }
    }
    PASS // Floating-point slack: fall back to the always-legal pass.
}

/// Uniform `f32` in `[0, 1)` from the top 24 bits of an RNG draw.
fn uniform(rng: &mut Rng) -> f32 {
    (rng.next_u64() >> 40) as f32 / (1u64 << 24) as f32
}

/// A standard-normal sample via Box–Muller.
fn gaussian(rng: &mut Rng) -> f32 {
    let u1 = uniform(rng).max(f32::MIN_POSITIVE);
    let u2 = uniform(rng);
    (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_round_trips() {
        let mut rng = Rng::new(7);
        let p = Policy::init(&mut rng);
        let restored = Policy::deserialize(&p.serialize()).unwrap();
        assert_eq!(p.buf, restored.buf);
    }

    #[test]
    fn deserialize_rejects_garbage() {
        assert!(Policy::deserialize("nope").is_err());
        // Right header, wrong shape.
        assert!(Policy::deserialize("overthrow-policy 1 1 1 1 0.0").is_err());
    }

    #[test]
    fn backward_matches_finite_difference() {
        // The whole reason we can hand-roll the gradient: check every weight's
        // analytic partial against a central finite difference of the loss.
        let mut rng = Rng::new(3);
        let base = Policy::init(&mut rng);

        let mut x = [0.0f32; INPUT];
        for xi in &mut x {
            *xi = gaussian(&mut rng);
        }
        let mut allowed = [false; HEAD];
        for &j in &[0usize, 2, 5, PASS] {
            allowed[j] = true;
        }
        let d = Decision {
            x,
            allowed,
            action: 2,
        };
        let coef = 0.7;

        let mut grads = Grads::zeros();
        base.backward_into(&d, coef, &mut grads);

        let eps = 1e-3;
        for (block, &size) in SIZES.iter().enumerate() {
            for i in 0..size {
                let mut probe = base.clone();
                let orig = probe.buf[block][i];
                probe.buf[block][i] = orig + eps;
                let lp = probe.loss(&d, coef);
                probe.buf[block][i] = orig - eps;
                let lm = probe.loss(&d, coef);
                let numeric = (lp - lm) / (2.0 * eps);
                let analytic = grads.buf[block][i];
                assert!(
                    (analytic - numeric).abs() < 1e-2 * (1.0 + analytic.abs()) + 1e-4,
                    "block {block} index {i}: analytic {analytic}, numeric {numeric}"
                );
            }
        }
    }

    #[test]
    fn masked_softmax_zeroes_illegal_and_sums_to_one() {
        let logits = [1.0; HEAD];
        let mut allowed = [false; HEAD];
        allowed[0] = true;
        allowed[PASS] = true;
        let p = softmax_masked(&logits, &allowed);
        assert!((p.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert_eq!(p[1], 0.0);
        assert!((p[0] - 0.5).abs() < 1e-6);
    }
}
