//! Tuning the engine's evaluation weights on recorded games ("Texel tuning"):
//! choose the weights whose evaluation best predicts the games' results.
//!
//! Every quiet position of every recorded game (no local board can be taken at
//! once) is paired with the game's result for X. The evaluation, squashed by a
//! logistic curve, should predict that result; the weights are improved by
//! coordinate search on the mean squared error.

use std::process::ExitCode;

use clap::Args;
use ulttt_db::Db;
use ulttt_engine::{Evaluator, Options, Weights};
use ulttt_game::{Player, Position, Variant};

#[derive(Args)]
pub struct TexelArgs {
    #[arg(short, long, default_value = "classic")]
    variant: Variant,
    /// Starting settings, as for `match`; only the weights are tuned.
    #[arg(short, long, default_value = "")]
    engine: String,
    /// Skip the first plies of each game (its random opening and the moves
    /// right after).
    #[arg(long, default_value_t = 8)]
    skip: usize,
    /// Rounds of coordinate search.
    #[arg(long, default_value_t = 20)]
    rounds: usize,
}

/// A quiet position and its game's result for X (1, 0.5 or 0).
struct Sample {
    pos: Position,
    result: f32,
}

/// Whether the player to move can take a local board with their move.
fn can_capture(pos: &Position, v: Variant) -> bool {
    let me = pos.to_move();
    let boards = pos.allowed_boards(v) & !(pos.owned_by(Player::X) | pos.owned_by(Player::O));
    (0..9u8).any(|b| boards >> b & 1 != 0 && pos.local_threats(me, b) != 0)
}

fn samples(db: &Db, v: Variant, skip: usize) -> Vec<Sample> {
    let mut out = Vec::new();
    for (opening, moves, score) in db.recorded_games(v).expect("read games") {
        let result = (f32::from(score) + 1.0) / 2.0;
        let mut pos = opening;
        for (i, &m) in moves.iter().enumerate() {
            if i >= skip && !pos.is_terminal(v) && !can_capture(&pos, v) {
                out.push(Sample { pos, result });
            }
            pos.play(m);
        }
    }
    out
}

/// The evaluation for X of every sample.
fn evals(w: &Weights, v: Variant, data: &[Sample]) -> Vec<f32> {
    let e = Evaluator::new(*w);
    data.iter()
        .map(|s| {
            let x = e.evaluate(&s.pos, v) as f32;
            if s.pos.to_move() == Player::X { x } else { -x }
        })
        .collect()
}

fn mse(evals: &[f32], data: &[Sample], k: f32) -> f64 {
    let sum: f64 = evals
        .iter()
        .zip(data)
        .map(|(&e, s)| {
            let p = 1.0 / (1.0 + (-k * e).exp());
            f64::from((s.result - p).powi(2))
        })
        .sum();
    sum / data.len() as f64
}

/// The logistic scale that best fits the evaluations, by golden-section
/// search on a log scale.
fn fit_k(evals: &[f32], data: &[Sample]) -> f32 {
    let (mut lo, mut hi) = (-12.0f32, 0.0f32);
    for _ in 0..40 {
        let a = lo + (hi - lo) * 0.382;
        let b = lo + (hi - lo) * 0.618;
        if mse(evals, data, a.exp()) < mse(evals, data, b.exp()) {
            hi = b;
        } else {
            lo = a;
        }
    }
    ((lo + hi) / 2.0).exp()
}

type Field = (&'static str, fn(&mut Weights) -> &mut f32);

const FIELDS: [Field; 12] = [
    ("global", |w| &mut w.global),
    ("local", |w| &mut w.local),
    ("majority", |w| &mut w.majority),
    ("tempo", |w| &mut w.tempo),
    ("line1", |w| &mut w.line1),
    ("line2", |w| &mut w.line2),
    ("center", |w| &mut w.center),
    ("corner", |w| &mut w.corner),
    ("edge", |w| &mut w.edge),
    ("base", |w| &mut w.base),
    ("fill", |w| &mut w.fill),
    ("prior", |w| &mut w.prior),
];

fn spec(w: &Weights) -> String {
    let mut w = *w;
    FIELDS
        .iter()
        .map(|(name, f)| format!("{name}={:.4}", *f(&mut w)))
        .collect::<Vec<_>>()
        .join(",")
}

pub fn run(db: &Db, args: &TexelArgs) -> ExitCode {
    let opts = match Options::parse(&args.engine) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let v = args.variant;
    let data = samples(db, v, args.skip);
    if data.is_empty() {
        eprintln!("no recorded games with moves for {v}; play some matches first");
        return ExitCode::FAILURE;
    }
    let mut w = opts.weights;
    // The logistic scale is fitted once to the starting weights; after that
    // the weights themselves carry the scale.
    let k = fit_k(&evals(&w, v, &data), &data);
    let mut best = mse(&evals(&w, v, &data), &data, k);
    println!("{} positions, k = {k:.3e}, start mse {best:.6}", data.len());
    let mut step = 0.2f32;
    for round in 1..=args.rounds {
        let mut improved = false;
        for (name, field) in FIELDS {
            if name == "majority" && v != Variant::Majority {
                continue;
            }
            for dir in [1.0 + step, 1.0 / (1.0 + step)] {
                let mut cand = w;
                *field(&mut cand) *= dir;
                let e = mse(&evals(&cand, v, &data), &data, k);
                if e < best {
                    best = e;
                    w = cand;
                    improved = true;
                    break;
                }
            }
        }
        println!("round {round}: mse {best:.6} step {step:.3}  {}", spec(&w));
        if !improved {
            step /= 2.0;
            if step < 0.01 {
                break;
            }
        }
    }
    println!("tuned: {}", spec(&w));
    ExitCode::SUCCESS
}
