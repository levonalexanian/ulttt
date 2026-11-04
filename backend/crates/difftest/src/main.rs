//! Differential testing against the Lean reference executable (`proofs/Main.lean`).
//!
//! 1. Rules: plays random games with the Rust engine and checks Lean agrees on
//!    every position: the legal moves, the board owners, the result, and
//!    whether each player may still win (`may_win`, proved sound in Lean).
//! 2. Values: picks random endgame positions and checks every Rust solver's value
//!    equals `Ulttt.value`, the minimax value defined in Lean.
//!
//! Usage: `ulttt-difftest <path to ulttt-ref> [games] [seed]`

use std::io::{BufRead, BufReader, Lines, Write};
use std::process::{Child, ChildStdout, Command, ExitCode, Stdio};
use std::thread;

use ulttt_game::random::Rng;
use ulttt_game::{Move, Moves, Outcome, Player, Position, Variant};
use ulttt_solver::Solver;

/// The description of a position in the reference executable's output format.
fn describe(v: Variant, pos: &Position) -> String {
    let legal: Vec<String> = Moves(pos.legal_moves(v)).map(|m| m.to_string()).collect();
    let owners: String = (0..9)
        .map(|b| pos.owner(b).map_or('.', Player::symbol))
        .collect();
    let result = match pos.result(v) {
        None => "-",
        Some(Outcome::Win(Player::X)) => "x",
        Some(Outcome::Win(Player::O)) => "o",
        Some(Outcome::Draw) => "d",
    };
    let may = |p| if pos.may_win(v, p) { "1" } else { "0" };
    format!(
        "{}|{owners}|{result}|{}{}",
        legal.join(","),
        may(Player::X),
        may(Player::O)
    )
}

fn play_all(moves: &[Move]) -> Position {
    let mut pos = Position::new();
    for &m in moves {
        pos.play(m);
    }
    pos
}

/// The number of positions in the full game tree below `pos`, or `None` if it
/// exceeds `limit`.
fn tree_size(pos: &Position, v: Variant, limit: u64) -> Option<u64> {
    let mut total = 1;
    for m in Moves(pos.legal_moves(v)) {
        total += tree_size(&pos.after(m), v, limit - total)?;
        if total > limit {
            return None;
        }
    }
    Some(total)
}

/// The reference executable, fed by a writer thread so pipes cannot deadlock.
struct Reference {
    child: Child,
    lines: Lines<BufReader<ChildStdout>>,
    writer: Option<thread::JoinHandle<()>>,
}

impl Reference {
    fn start(path: &str, input: Vec<String>) -> Reference {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("failed to start the reference executable");
        let mut stdin = child.stdin.take().expect("stdin");
        let writer = thread::spawn(move || {
            for line in input {
                stdin
                    .write_all(line.as_bytes())
                    .expect("write to reference");
            }
        });
        let lines = BufReader::new(child.stdout.take().expect("stdout")).lines();
        Reference {
            child,
            lines,
            writer: Some(writer),
        }
    }

    fn next_line(&mut self) -> String {
        self.lines
            .next()
            .expect("reference ended early")
            .expect("read from reference")
    }

    fn finish(mut self) {
        if let Some(w) = self.writer.take() {
            w.join().expect("writer thread");
        }
        self.child.wait().expect("reference exit");
    }
}

impl Drop for Reference {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn move_list(moves: &[Move]) -> String {
    moves
        .iter()
        .map(|m| m.to_string())
        .collect::<Vec<_>>()
        .join(" ")
}

fn check_rules(reference: &str, games: usize, rng: &mut Rng) -> bool {
    let cases: Vec<(Variant, Vec<Move>)> = (0..games)
        .map(|i| {
            let v = Variant::ALL[i % Variant::ALL.len()];
            (v, rng.game(v))
        })
        .collect();
    let input = cases
        .iter()
        .map(|(v, moves)| format!("{v} {}\n", move_list(moves)))
        .collect();
    let mut lean = Reference::start(reference, input);
    let mut positions = 0usize;
    for (i, (v, moves)) in cases.iter().enumerate() {
        let mut pos = Position::new();
        for k in 0..=moves.len() {
            if k > 0 {
                pos.play(moves[k - 1]);
            }
            let want = describe(*v, &pos);
            let got = lean.next_line();
            if got != want {
                eprintln!(
                    "MISMATCH in game {i} ({v}) after {k} moves: {:?}",
                    &moves[..k]
                );
                eprintln!("  rust: {want}");
                eprintln!("  lean: {got}");
                return false;
            }
            positions += 1;
        }
    }
    lean.finish();
    println!("rules ok: {games} games, {positions} positions agree");
    true
}

fn check_values(reference: &str, count: usize, rng: &mut Rng) -> bool {
    // Endgame positions whose full game tree is small enough for Lean's minimax.
    let mut cases: Vec<(Variant, Vec<Move>)> = Vec::new();
    while cases.len() < count {
        let v = Variant::ALL[cases.len() % Variant::ALL.len()];
        let game = rng.game(v);
        let back = 1 + rng.below(12.min(game.len()));
        let moves = game[..game.len() - back].to_vec();
        if tree_size(&play_all(&moves), v, 3_000).is_some() {
            cases.push((v, moves));
        }
    }
    let input = cases
        .iter()
        .map(|(v, moves)| format!("value {v} {}\n", move_list(moves)))
        .collect();
    let mut lean = Reference::start(reference, input);
    // Every solver, each kept across positions as in real use.
    let mut solvers: Vec<Box<dyn Solver>> = ["alphabeta", "search", "search:threads=2", "dfpn"]
        .iter()
        .map(|name| ulttt_solver::by_name(name).expect("known solver"))
        .collect();
    for (v, moves) in &cases {
        let got = lean.next_line();
        for solver in &mut solvers {
            let want = solver.solve(&play_all(moves), *v).expect("no node limit") as i8;
            if got != want.to_string() {
                eprintln!("VALUE MISMATCH ({v}, {}) after {:?}", solver.name(), moves);
                eprintln!("  rust: {want}");
                eprintln!("  lean: {got}");
                return false;
            }
        }
    }
    lean.finish();
    println!(
        "values ok: {count} endgames agree for {} solvers",
        solvers.len()
    );
    true
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let Some(reference) = args.get(1) else {
        eprintln!("usage: ulttt-difftest <path to ulttt-ref> [games] [seed]");
        return ExitCode::FAILURE;
    };
    let games: usize = args.get(2).map_or(3000, |s| s.parse().expect("games"));
    let seed: u64 = args.get(3).map_or(1, |s| s.parse().expect("seed"));
    println!("seed {seed}");

    let mut rng = Rng::new(seed);
    if check_rules(reference, games, &mut rng) && check_values(reference, games / 30, &mut rng) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
