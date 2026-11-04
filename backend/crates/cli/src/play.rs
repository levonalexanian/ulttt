//! Matches between playing-engine configurations, with results in SQLite.

use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use clap::Args;
use ulttt_db::{Db, MatchGame, MatchSummary, NewMatch};
use ulttt_engine::Agent;
use ulttt_game::random::Rng;
use ulttt_game::{Move, Outcome, Player, Position, Variant};

#[derive(Args)]
pub struct MatchArgs {
    /// Engine A: alpha-beta settings as in `Options::parse` (e.g.
    /// `tempo=0,tt=16`, empty for the defaults), or `mcts[:<settings>]`.
    #[arg(short = 'a', long, default_value = "")]
    engine_a: String,
    /// Settings of engine B.
    #[arg(short = 'b', long, default_value = "")]
    engine_b: String,
    #[arg(short, long, default_value = "classic")]
    variant: Variant,
    /// Nodes each engine may search per move.
    #[arg(long, default_value_t = 20_000)]
    nodes: u64,
    /// Also limit each move to this many milliseconds.
    #[arg(long)]
    ms: Option<u64>,
    /// Number of openings; each is played twice, once with each engine as X.
    #[arg(short = 'n', long, default_value_t = 50)]
    openings: usize,
    /// Random moves played to make each opening.
    #[arg(long, default_value_t = 4)]
    opening_plies: usize,
    #[arg(short, long, default_value_t = 1)]
    seed: u64,
    /// Games played in parallel.
    #[arg(short, long, default_value_t = 1)]
    jobs: usize,
    /// Git revision of the code, recorded with the match.
    #[arg(long, default_value = "")]
    rev: String,
    #[arg(long, default_value = "")]
    notes: String,
    /// Stop early by a sequential probability ratio test of "A is `elo1`
    /// stronger" against "A is `elo0` stronger" (5% error each way), given as
    /// `elo0,elo1`, e.g. `0,15`.
    #[arg(long)]
    sprt: Option<String>,
}

/// Parses SPRT bounds `elo0,elo1`.
fn parse_sprt(s: &str) -> Result<(f64, f64), String> {
    let bad = || format!("expected --sprt elo0,elo1, got {s:?}");
    let (a, b) = s.split_once(',').ok_or_else(bad)?;
    let e0: f64 = a.trim().parse().map_err(|_| bad())?;
    let e1: f64 = b.trim().parse().map_err(|_| bad())?;
    if e0 < e1 { Ok((e0, e1)) } else { Err(bad()) }
}

/// The log-likelihood ratio of "A is `elo1` stronger" against "A is `elo0`
/// stronger" given the games' scores, in the normal approximation.
pub fn llr(scores: &[f64], elo0: f64, elo1: f64) -> f64 {
    let n = scores.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let mean = scores.iter().sum::<f64>() / n;
    let var = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n;
    if var == 0.0 {
        return 0.0;
    }
    let expected = |e: f64| 1.0 / (1.0 + 10f64.powf(-e / 400.0));
    let (s0, s1) = (expected(elo0), expected(elo1));
    n * (s1 - s0) * (2.0 * mean - s0 - s1) / (2.0 * var)
}

/// Plays one game from `opening` and returns its outcome and length.
fn play_game(
    v: Variant,
    opening: &Position,
    x: &str,
    o: &str,
    nodes: u64,
    ms: Option<u64>,
) -> (Outcome, Vec<Move>) {
    let mut engines: [Box<dyn Agent>; 2] = [
        ulttt_engine::agent(v, x).expect("checked"),
        ulttt_engine::agent(v, o).expect("checked"),
    ];
    let mut pos = *opening;
    let mut moves = Vec::new();
    while !pos.is_terminal(v) {
        let side = pos.to_move().index();
        if let Some(ms) = ms {
            let start = Instant::now();
            let total = Duration::from_millis(ms).as_secs_f32();
            engines[side].set_clock(Some(Box::new(move || {
                start.elapsed().as_secs_f32() / total
            })));
        }
        let choice = engines[side].best_move(&pos, nodes).expect("game not over");
        pos.play(choice.mv);
        moves.push(choice.mv);
    }
    (pos.outcome(v), moves)
}

/// `engine` or `engine:<settings>`, as stored in the database.
fn label(spec: &str) -> String {
    if spec.is_empty() {
        "engine".into()
    } else if spec.starts_with("mcts") {
        spec.into()
    } else {
        format!("engine:{spec}")
    }
}

/// The Elo difference for a score fraction, and a 95% margin from the games'
/// scores (each 1, 0.5 or 0).
pub fn elo(scores: &[f64]) -> (f64, f64) {
    let n = scores.len() as f64;
    let mean = scores.iter().sum::<f64>() / n;
    let var = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>() / n;
    let clamped = mean.clamp(0.001, 0.999);
    let diff = -400.0 * (1.0 / clamped - 1.0).log10();
    // Derivative of the Elo curve at `mean`, times the standard error.
    let slope = 400.0 / (std::f64::consts::LN_10 * clamped * (1.0 - clamped));
    (diff, 1.96 * slope * (var / n).sqrt())
}

pub fn run(db: &Db, args: &MatchArgs) -> ExitCode {
    let sprt = match args.sprt.as_deref().map(parse_sprt) {
        None => None,
        Some(Ok(bounds)) => Some(bounds),
        Some(Err(e)) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };
    let v = args.variant;
    let (a, b) = (args.engine_a.as_str(), args.engine_b.as_str());
    match (ulttt_engine::agent(v, a), ulttt_engine::agent(v, b)) {
        (Ok(_), Ok(_)) => {}
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    }
    let mut rng = Rng::new(args.seed);
    let mut openings = Vec::new();
    while openings.len() < args.openings {
        if let Some(p) = rng.position(v, args.opening_plies) {
            openings.push(p);
        }
    }
    let (label_a, label_b) = (label(&args.engine_a), label(&args.engine_b));
    let match_id = db
        .start_match(&NewMatch {
            variant: v,
            engine_a: &label_a,
            engine_b: &label_b,
            node_limit: args.nodes,
            ms: args.ms,
            git_rev: &args.rev,
            notes: &args.notes,
        })
        .expect("start match");
    let games = 2 * openings.len();
    let next = AtomicUsize::new(0);
    let stop = AtomicBool::new(false);
    let (tx, rx) = mpsc::channel();
    let mut scores = Vec::with_capacity(games);
    let start = Instant::now();
    std::thread::scope(|scope| {
        for _ in 0..args.jobs.max(1) {
            let (tx, next, openings, stop) = (tx.clone(), &next, &openings, &stop);
            scope.spawn(move || {
                loop {
                    let idx = next.fetch_add(1, Ordering::Relaxed);
                    if idx >= games || stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let opening = openings[idx / 2];
                    let a_is_x = idx % 2 == 0;
                    let (x, o) = if a_is_x { (a, b) } else { (b, a) };
                    let t = Instant::now();
                    let (outcome, moves) = play_game(v, &opening, x, o, args.nodes, args.ms);
                    let a_player = if a_is_x { Player::X } else { Player::O };
                    let score = match outcome {
                        Outcome::Win(p) if p == a_player => 1,
                        Outcome::Win(_) => -1,
                        Outcome::Draw => 0,
                    };
                    let game = MatchGame {
                        idx,
                        opening,
                        a_is_x,
                        score,
                        plies: moves.len() as u32,
                        micros: t.elapsed().as_micros() as u64,
                        moves,
                    };
                    let _ = tx.send(game);
                }
            });
        }
        drop(tx);
        for g in rx {
            db.record_game(match_id, &g).expect("record game");
            scores.push((f64::from(g.score) + 1.0) / 2.0);
            if scores.len() % 20 == 0 {
                let (d, m) = elo(&scores);
                eprintln!("{}/{games} games: A {d:+.0} ± {m:.0} Elo", scores.len());
            }
            if let Some((e0, e1)) = sprt
                && !stop.load(Ordering::Relaxed)
            {
                let l = llr(&scores, e0, e1);
                if l.abs() >= 19f64.ln() {
                    let verdict = if l > 0.0 { "H1 (A stronger)" } else { "H0" };
                    eprintln!(
                        "SPRT: {verdict} accepted after {} games, LLR {l:.2}",
                        scores.len()
                    );
                    stop.store(true, Ordering::Relaxed);
                }
            }
        }
    });
    db.finish_match(match_id).expect("finish match");
    let summary = db
        .match_summaries(usize::MAX >> 1)
        .expect("read matches")
        .into_iter()
        .find(|m| m.id == match_id)
        .expect("just played");
    print_summary(&summary);
    println!("{:.0}s", start.elapsed().as_secs_f64());
    ExitCode::SUCCESS
}

fn print_summary(m: &MatchSummary) {
    let mut scores = vec![1.0; m.a_wins as usize];
    scores.extend(std::iter::repeat_n(0.5, m.draws as usize));
    scores.extend(std::iter::repeat_n(0.0, m.b_wins as usize));
    let elo = if scores.is_empty() {
        String::new()
    } else {
        let (d, margin) = elo(&scores);
        format!("{d:+5.0} ± {margin:3.0} Elo")
    };
    let limit = match m.ms {
        Some(ms) => format!("{ms} ms"),
        None => format!("{} nodes", m.node_limit),
    };
    println!(
        "#{:<4} {:<8} {limit:>14}  A +{} ={} -{}  {elo}  A {} vs B {}",
        m.id, m.variant, m.a_wins, m.draws, m.b_wins, m.engine_a, m.engine_b
    );
}

/// Runs the engine once on each of `count` positions per game phase, with a
/// fresh engine each time, and prints positions per second and a checksum of
/// the chosen moves.
pub fn bench(spec: &str, v: Variant, nodes: u64, count: usize) -> ExitCode {
    if let Err(e) = ulttt_engine::agent(v, spec) {
        eprintln!("{e}");
        return ExitCode::FAILURE;
    }
    let mut rng = Rng::new(7);
    let mut checksum = 0u64;
    let (mut total_nodes, mut total_secs) = (0u64, 0f64);
    for plies in [6, 20, 34] {
        let mut done = 0;
        let (mut n, mut secs) = (0u64, 0f64);
        while done < count {
            let Some(pos) = rng.position(v, plies) else {
                continue;
            };
            let mut agent = ulttt_engine::agent(v, spec).expect("checked");
            let t = Instant::now();
            let c = agent.best_move(&pos, nodes).expect("not over");
            secs += t.elapsed().as_secs_f64();
            n += c.nodes;
            checksum = checksum.wrapping_mul(31).wrapping_add(u64::from(c.mv) + 1);
            done += 1;
        }
        println!(
            "after {plies:>2} moves: {:>6.0}k positions/s ({n} positions, {secs:.2}s)",
            n as f64 / secs / 1000.0
        );
        total_nodes += n;
        total_secs += secs;
    }
    println!(
        "total: {:.0}k positions/s, move checksum {checksum:016x}",
        total_nodes as f64 / total_secs / 1000.0
    );
    ExitCode::SUCCESS
}

/// Prints the most recent matches.
pub fn list(db: &Db, limit: usize) -> ExitCode {
    for m in db.match_summaries(limit).expect("read matches") {
        print_summary(&m);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elo_of_scores() {
        assert_eq!(elo(&[1.0, 0.0]).0, 0.0);
        let (d, m) = elo(&[1.0, 1.0, 1.0, 0.0]);
        assert!((d - 190.8).abs() < 0.1, "{d}");
        assert!(m > 0.0);
    }

    #[test]
    fn sprt_llr() {
        // Winning 60% clearly favors +15 over 0; losing 60% favors 0.
        let mut good = vec![1.0; 600];
        good.extend(vec![0.0; 400]);
        assert!(llr(&good, 0.0, 15.0) > 19f64.ln());
        let mut bad = vec![1.0; 400];
        bad.extend(vec![0.0; 600]);
        assert!(llr(&bad, 0.0, 15.0) < -(19f64.ln()));
        assert_eq!(parse_sprt("0,15"), Ok((0.0, 15.0)));
        assert!(parse_sprt("15,0").is_err());
    }
}
